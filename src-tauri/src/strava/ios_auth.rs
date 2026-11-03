use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use block2::RcBlock;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, AnyThread, DefinedClass,
    MainThreadOnly,
};
use objc2_authentication_services::{
    ASWebAuthenticationPresentationContextProviding, ASWebAuthenticationSession,
};
use objc2_foundation::{MainThreadMarker, NSError, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{UIApplication, UIWindow};

use super::{failure, Result};

define_class!(
    #[unsafe(super = NSObject)]
    #[name = "RunalyticsStravaPresentation"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<UIWindow>]
    struct Presentation;

    unsafe impl NSObjectProtocol for Presentation {}
    unsafe impl ASWebAuthenticationPresentationContextProviding for Presentation {}

    impl Presentation {
        #[unsafe(method(presentationAnchorForWebAuthenticationSession:))]
        fn anchor(&self, _session: &ASWebAuthenticationSession) -> *mut UIWindow {
            Retained::as_ptr(self.ivars()) as *mut UIWindow
        }
    }
);

thread_local! {
    static SESSION: RefCell<Option<(Retained<ASWebAuthenticationSession>, Retained<Presentation>)>> = const { RefCell::new(None) };
}

pub async fn authorize(app: &tauri::AppHandle, address: &str) -> Result<String> {
    let address = address.to_string();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let callback_sender = sender.clone();
    app.run_on_main_thread(move || {
        let start = || -> Result<()> {
            let main = MainThreadMarker::new()
                .ok_or_else(|| failure("Strava sign-in needs the main thread."))?;
            #[allow(deprecated)]
            let windows = UIApplication::sharedApplication(main).windows();
            let window = windows
                .iter()
                .find(|window| window.isKeyWindow())
                .ok_or_else(|| {
                    failure("Bring Runalytics to the foreground before connecting Strava.")
                })?;
            let delegate = Presentation::alloc(main).set_ivars(window);
            let delegate: Retained<Presentation> = unsafe { msg_send![super(delegate), init] };
            let completion = RcBlock::new(move |url: *mut NSURL, _error: *mut NSError| {
                let outcome = unsafe { url.as_ref() }
                    .and_then(|url| url.absoluteString())
                    .map(|url| url.to_string())
                    .ok_or_else(|| failure("Strava sign-in was cancelled or could not finish."));
                if let Some(sender) = callback_sender.lock().unwrap().take() {
                    let _ = sender.send(outcome);
                }
            });
            let url = NSURL::URLWithString(&NSString::from_str(&address))
                .ok_or_else(|| failure("Invalid Strava sign-in address."))?;
            #[allow(deprecated)]
            let session = unsafe {
                ASWebAuthenticationSession::initWithURL_callbackURLScheme_completionHandler(
                    ASWebAuthenticationSession::alloc(),
                    &url,
                    Some(&NSString::from_str("com.fradone.runalytics")),
                    RcBlock::as_ptr(&completion) as *mut _,
                )
            };
            unsafe {
                session.setPresentationContextProvider(Some(ProtocolObject::from_ref(&*delegate)));
            }
            if !unsafe { session.start() } {
                return Err(failure(
                    "Could not start Strava sign-in. Try again from the foreground.",
                ));
            }
            SESSION.with(|slot| *slot.borrow_mut() = Some((session, delegate)));
            Ok(())
        };
        if let Err(error) = start() {
            if let Some(sender) = sender.lock().unwrap().take() {
                let _ = sender.send(Err(error));
            }
        }
    })
    .map_err(|_| failure("Could not open native Strava sign-in."))?;
    let result = tokio::time::timeout(Duration::from_secs(300), receiver).await;
    cancel(app)?;
    result
        .map_err(|_| failure("Strava sign-in timed out. Try again."))?
        .map_err(|_| failure("Strava sign-in was interrupted."))?
}

pub fn cancel(app: &tauri::AppHandle) -> Result<()> {
    app.run_on_main_thread(|| {
        SESSION.with(|slot| {
            if let Some((session, _delegate)) = slot.borrow_mut().take() {
                unsafe {
                    session.cancel();
                }
            }
        });
    })
    .map_err(|_| failure("Could not close Strava sign-in."))
}
