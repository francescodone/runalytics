try {
  const theme = localStorage.getItem("runalytics.theme") || "system";
  const resolved = theme === "system" ? (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : theme;
  document.documentElement.dataset.deskTheme = resolved;
  document.documentElement.dataset.deskPalette = localStorage.getItem("runalytics.palette") || "amethyst";
  document.documentElement.style.colorScheme = resolved;
} catch {}