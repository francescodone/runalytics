import "./lib/theme";
import "./lib/navigation";
import { initSyncIndicator } from "./lib/sync";
import { hydrateIcons } from "./lib/icons";

hydrateIcons();
document.getElementById("support-version")!.textContent = `Runalytics ${__APP_VERSION__}`;
initSyncIndicator();