# Runalytics Credential Setup

Do not put actual credentials in this file, commit them, or send them in chat.
The Supabase integration is under development and is not ready for real user
data. Project linking does not deploy migrations or configure sign-in.

## Prerequisites

Complete these before working through the relevant provider setup. Dashboard
labels and eligibility requirements can change, so check each provider's
current documentation when registering.

### Supabase and local development

- A Supabase account and a created Runalytics project, with permission to
   manage authentication providers and project settings. This project is
   already linked locally; linking does not apply migrations to it.
- For local database tests: Docker with its daemon running and the Supabase CLI
   installed. A cloud project or provider credentials are not needed to run the
   existing local database tests.

### Google

- A Google account and a created or accessible Google Cloud project, with
   permission to configure Google Auth Platform and create OAuth clients. A
   separate paid developer program is not required for Google sign-in.
- A support email and developer contact email for the consent screen. For a
   public launch, prepare the app homepage, privacy policy, and terms URLs as
   required by Google, plus domain/brand verification if requested.
- For an external app in testing mode, the Google accounts of the people you
   want to add as test users. Moving to production may require verification;
   check the Google Auth Platform publishing status and requirements.

### Apple

- An Apple Account enrolled in the paid Apple Developer Program, or access to
   an enrolled organization's team, with permission to manage App IDs and
   signing. Confirm the current membership price and terms with Apple.
- An existing or registrable explicit Runalytics App ID and the actual bundle
   identifier for each signed target. The repository currently configures
   `com.fradone.runalytics`; confirm the signed targets before changing Apple
   settings.
- A signed macOS/iOS build with the Sign in with Apple capability and suitable
   provisioning is needed for end-to-end testing. The capability and sign-in flow
   are not yet implemented in Runalytics.

### Strava

- A Strava account and an existing registered API application. Strava's current
   documentation requires a Strava subscription to create an application; an
   ordinary Runalytics user does not need to create their own developer app.
- Permission to manage the application's API settings, plus sufficient athlete
   capacity for the intended testers. Strava review is required to scale beyond
   the capacity available in the API dashboard.

## Supabase

1. In the Supabase dashboard, select the Runalytics project.
2. Under **Authentication > Sign In / Providers**, configure the Apple and Google
   providers as described below. Leave email sign-in disabled for the initial
   release unless a production SMTP service is configured.
3. Under **Project Settings > API Keys**, the project URL and publishable key
   identify the project to the app. A place to configure these in Runalytics
   has not yet been implemented. The publishable key is not a user credential;
   access still requires correct row-level security.
4. Never place a secret key, service-role key, database password, access token,
   or provider secret in the desktop/iOS application or in a tracked file.
   Supabase provides its backend keys to Edge Functions at runtime; do not
   manually copy them into client configuration.

## Google sign-in

The planned flow opens the system browser and uses Supabase to exchange the
Google authorization code. For this flow, create a **Web application** OAuth
client, even though Runalytics runs on macOS and iOS.

### 1. Configure the consent screen

1. Open [Google Cloud Console](https://console.cloud.google.com/) and create or
   select the Google Cloud project that will own Runalytics sign-in.
2. Open **Google Auth Platform > Branding**. Enter `Runalytics` as the app name,
   choose a user support email, and provide the required developer contact
   email. Add a homepage, privacy policy, and terms URLs when available and
   required for publishing or verification. Complete Google's domain and brand
   verification if prompted.
3. Under **Audience**, choose **External** if people outside your Google
   Workspace organization need to sign in. While the application is in
   **Testing**, add each tester's Google account under **Test users**, including
   your own. Publishing and any required verification come later.
4. Under **Data Access**, configure only `openid`, `userinfo.email`, and
   `userinfo.profile` for sign-in. Supabase's guide says `openid` may need to be
   added manually; the two userinfo scopes are generally present by default.
   Do not request additional Google API scopes unless a feature needs them.

### 2. Create the OAuth client

1. In Supabase, open **Authentication > Sign In / Providers > Google** and copy
   the **Callback URL** displayed for the linked project. Keep this page open.
2. In Google Cloud, open **Google Auth Platform > Clients > Create client**.
3. Select **Web application** and give it a recognizable name, such as
   `Runalytics Supabase Auth`.
4. Under **Authorized redirect URIs**, add the exact Supabase callback URL from
   step 1, including its full `https://` URL and `/auth/v1/callback` path. This
   is Google's return to Supabase, not the later return from Supabase to the
   installed app. For local Supabase testing, its documented separate callback
   is `http://127.0.0.1:54321/auth/v1/callback`; add it to a development client
   when local provider testing is needed.
5. **Authorized JavaScript origins** accepts origins only, without paths. Follow
   Google's and Supabase's instructions for any actual web origin you use; do
   not invent a Runalytics website or native callback origin for this setting.
6. Create the client. Google displays a **Client ID** and **Client secret**.

### 3. Connect it to Supabase

1. Return to the Supabase **Google** provider panel. Paste the Google Web
   **Client ID** into **Client IDs** and its **Client secret** into **Client
   Secret**. Enable the provider and save. Enter the secret directly in the
   dashboard, not in this file, the repository, or chat.
2. Keep nonce checks enabled; do not change **Skip nonce check** just to get a
   test working. The planned browser flow does not need a separate Google iOS
   OAuth client. A native Google SDK flow would have different requirements.
3. The app's return URL and Supabase redirect allowlist entry are not yet
   implemented. Do not guess them. Once implemented, add the exact documented
   return URL under Supabase **Authentication > URL Configuration > Redirect
   URLs**; it is separate from Google's Supabase callback above.

**Status:** Provider configuration can be prepared now, but Runalytics cannot
complete sign-in until its native callback and session handling are implemented.
If Google reports `redirect_uri_mismatch`, compare the full Google authorized
redirect URI with the callback displayed in the Supabase Google provider panel.

## Apple sign-in

Runalytics plans to use **native Sign in with Apple** on macOS and iOS. An
Apple Developer account with access to the team's identifiers is needed. The
repository's configured application identifier is `com.fradone.runalytics`;
confirm that each actual signed macOS/iOS target uses that identifier before
registering it. Development or other targets may use different bundle IDs.

### 1. Enable the Apple App ID

1. Sign in to [Apple Developer](https://developer.apple.com/account/) with an
   account that can manage the correct team. Open **Certificates, Identifiers &
   Profiles > Identifiers**, then filter by **App IDs**.
2. Find the existing Runalytics App ID matching the actual bundle ID. Reuse it;
   do not create a duplicate or change a shipping app's identifier. If it does
   not exist, an authorized team member must register an explicit App ID for
   the intended bundle ID.
3. Open the App ID, enable **Sign in with Apple** under **Capabilities**, and
   save. If Apple asks whether it is a primary or grouped App ID, check your
   existing app grouping before changing it; grouping affects Apple identity
   across apps. Supabase's documentation says to leave the server-to-server
   notification endpoint blank because it is not currently supported there.
4. The Runalytics target also needs the matching native capability and valid
   signing/provisioning. That application-side work is not implemented yet;
   enabling the capability in the Apple portal alone does not finish setup.

### 2. Configure Supabase

1. Open **Authentication > Sign In / Providers > Apple** in the Supabase
   project.
2. Under **Client IDs**, add the actual bundle ID for each signed Runalytics
   target that will use native Apple sign-in, including any distinct development
   identifier. For the identifier currently configured in the repository,
   that value is `com.fradone.runalytics`; confirm the signed targets first.
3. Enable and save the provider. Native Apple sign-in does not require a
   Services ID, a `.p8` signing key, or the six-month web secret rotation.
   Keep those web-only settings out of scope unless a web/non-native Apple flow
   is added later.

**Status:** The App ID and Supabase provider can be prepared now, but Runalytics
does not yet implement native Apple sign-in or the required target capability.
After implementation, test using an appropriately signed build and an Apple
account on a supported device. Apple may provide a user's name only on the first
authorization, and users can choose to hide their email.

### Optional: Apple web sign-in later

Only if a browser-based Apple flow is added, create an Apple **Services ID**
associated with the App ID, configure its website domain and exact Supabase
callback URL, and create an Apple Sign in with Apple signing key. Supabase's
Apple guide explains how to generate the web client secret from the Team ID,
Key ID, Services ID, and `.p8` key. Enter the resulting secret only in the
Supabase Apple provider panel; store the `.p8` securely outside this repository.
Rotate the web client secret at least every six months. If both web and native
flows are used, Supabase requires the Services ID first in **Client IDs**,
followed by the native bundle IDs.

## Shared Strava application

Keep the existing application under **Strava > Settings > My API Application**.
Check its athlete capacity and arrange review for broader access. The shared
Client ID and Client Secret will be configured as server-side Supabase Edge
Function secrets once the Strava broker and callback URL are implemented. The
callback domain shown by that implementation must be entered in Strava's API
settings. Neither the secret names nor a final callback URL exist yet; do not
guess them or put the secret in the app or a source file.

## Future encryption and backups

The server-side encryption key, rotation process, independent backup location,
and recovery procedure have not been implemented. They must be established
before storing real plans or provider tokens. Keep the encryption key separately
from the encrypted backup; losing it may make encrypted data unrecoverable.

## References

- [Supabase Google sign-in](https://supabase.com/docs/guides/auth/social-login/auth-google)
- [Supabase Apple sign-in](https://supabase.com/docs/guides/auth/social-login/auth-apple)
- [Supabase Edge Function secrets](https://supabase.com/docs/guides/functions/secrets)
- [Google OAuth consent configuration](https://developers.google.com/workspace/guides/configure-oauth-consent)
- [Apple Developer Program](https://developer.apple.com/programs/)
- [Strava API settings and setup](https://developers.strava.com/docs/getting-started/)