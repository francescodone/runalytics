import { initializeAccount } from "./profiles";

void initializeAccount().then(async () => {
  const { startApp } = await import("./main");
  startApp();
}).catch(async error => {
  const { showStartupError } = await import("./main");
  showStartupError(String(error));
});
