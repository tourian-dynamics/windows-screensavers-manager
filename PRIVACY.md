# rSaver Privacy Policy

**rSaver (Windows Screensavers Manager)** is built with a commitment to user privacy. As a local terminal application, rSaver does not track, collect, or transmit your personal data.

---

## 1. Data Collection & Telemetry
* **No Telemetry**: rSaver does not include any analytics, crash reporting platforms, or tracking metrics.
* **No Personal Data**: rSaver does not collect or request any personal identifiers, email addresses, or system credentials.

---

## 2. Network Connections
rSaver only makes network requests in the following developer-initiated contexts:
1. **Catalog Feeds**: When launched, rSaver connects to the URLs defined in your `feed_urls` configuration (by default, the official GitHub repository `registry.json`) to fetch the list of available curated screensavers.
2. **Screensaver Downloads**: When you explicitly request to download/install an online screensaver, rSaver connects directly to the screensaver's host URL to download the binary.

All network requests are standard, direct HTTPS client-to-server connections. No cookies, session tracking, or profiling is performed.

---

## 3. Local Data Storage
All data managed by rSaver is kept strictly on your local machine under your user profile directory:
* **Configurations**: Stored at `%APPDATA%\rSaver\config.yaml`
* **Log Files**: Diagnostics are logged locally to `%APPDATA%\rSaver\rSaver.log`
* **Downloaded Screensavers**: Saved locally under `%APPDATA%\rSaver\screensavers\`

rSaver does not upload your local configuration files or selection history to any remote server.

---

## 4. Third-Party Screensavers
If you configure custom feed URLs (`feed_urls`), downloading a screensaver from those feeds will connect to whatever third-party server hosting the executable. We recommend only using trusted catalog URLs.
