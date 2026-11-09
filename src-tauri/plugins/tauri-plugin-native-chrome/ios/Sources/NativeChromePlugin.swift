import Tauri
import UIKit
import WebKit

private struct Tab: Decodable {
    let id: String
    let title: String
}

private struct Snapshot: Decodable {
    let protocolVersion: Int
    let session: String
    let revision: Int
    let tabs: [Tab]
    let selected: String?
    let theme: String
}

private final class ChromeView: UIView {
    let tabBar = UITabBar()

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        addSubview(tabBar)
        accessibilityIdentifier = "runalytics.nativeChrome"
        tabBar.accessibilityIdentifier = "runalytics.workspaceTabs"
    }

    required init?(coder: NSCoder) { nil }

    override func layoutSubviews() {
        super.layoutSubviews()
        let height = tabBar.sizeThatFits(bounds.size).height
        tabBar.frame = CGRect(x: 0, y: bounds.height - height, width: bounds.width, height: height)
    }

    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        guard !tabBar.isHidden, tabBar.frame.contains(point) else { return nil }
        return tabBar.hitTest(convert(point, to: tabBar), with: event)
    }
}

final class NativeChromePlugin: Plugin, UITabBarDelegate {
    private weak var webview: WKWebView?
    private var chrome: ChromeView?
    private var snapshot: Snapshot?
    private var navigationObserver: NSKeyValueObservation?

    override func load(webview: WKWebView) {
        self.webview = webview
        navigationObserver = webview.observe(\.isLoading, options: [.new]) { [weak self] _, change in
            guard change.newValue == true else { return }
            DispatchQueue.main.async { self?.removeChrome() }
        }
    }

    @objc public func attach(_ invoke: Invoke) throws {
        let next = try invoke.parseArgs(Snapshot.self)
        let ids = ["training", "journal", "health"]
        guard next.protocolVersion == 1, !next.session.isEmpty, next.revision >= 0,
              next.tabs.map(\.id) == ids, next.tabs.allSatisfy({ !$0.title.isEmpty }),
              next.selected == nil || ids.contains(next.selected!),
              ["light", "dark"].contains(next.theme) else {
            invoke.reject("Invalid native chrome snapshot")
            return
        }
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { invoke.reject("Native chrome is unavailable"); return }
            guard UIDevice.current.userInterfaceIdiom == .phone,
                  let webview = self.webview, let host = webview.superview,
                  webview.window != nil else {
                invoke.reject("Native chrome requires an attached iPhone WebView")
                return
            }
            if let current = self.snapshot,
               current.session != next.session || next.revision <= current.revision {
                invoke.reject("Stale native chrome snapshot")
                return
            }
            let view = self.chrome ?? ChromeView(frame: host.bounds)
            view.frame = host.bounds
            view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            view.overrideUserInterfaceStyle = next.theme == "dark" ? .dark : .light
            view.tabBar.delegate = self
            let symbols = ["calendar", "figure.run", "heart"]
            view.tabBar.items = next.tabs.enumerated().map { index, tab in
                let item = UITabBarItem(title: tab.title, image: UIImage(systemName: symbols[index]), tag: index)
                item.accessibilityIdentifier = "runalytics.tab.\(tab.id)"
                return item
            }
            view.tabBar.selectedItem = next.selected.flatMap { selected in
                ids.firstIndex(of: selected).flatMap { view.tabBar.items?[$0] }
            }
            self.snapshot = next
            self.chrome = view
            host.addSubview(view)
            view.setNeedsLayout()
            view.layoutIfNeeded()
            invoke.resolve([
                "protocolVersion": 1,
                "session": next.session,
                "revision": next.revision,
                "bottomInset": view.tabBar.frame.height
            ])
        }
    }

    @objc public func detach(_ invoke: Invoke) {
        DispatchQueue.main.async { [weak self] in
            self?.removeChrome()
            invoke.resolve()
        }
    }

    func tabBar(_ tabBar: UITabBar, didSelect item: UITabBarItem) {
        guard let snapshot = snapshot, snapshot.tabs.indices.contains(item.tag) else { return }
        let selected = snapshot.selected.flatMap { selected in snapshot.tabs.firstIndex { $0.id == selected } }
        tabBar.selectedItem = selected.flatMap { tabBar.items?[$0] }
        trigger("intent", data: [
            "session": snapshot.session,
            "revision": snapshot.revision,
            "eventId": UUID().uuidString,
            "action": "navigate",
            "workspace": snapshot.tabs[item.tag].id
        ])
    }

    private func removeChrome() {
        chrome?.removeFromSuperview()
        chrome = nil
        snapshot = nil
    }
}

@_cdecl("init_plugin_native_chrome")
public func initPlugin() -> Plugin {
    NativeChromePlugin()
}