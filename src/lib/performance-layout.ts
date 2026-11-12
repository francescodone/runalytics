export const performanceLayout = `
  <section id="empty" hidden></section>
  <div id="dash" hidden>
    <section id="status" class="panel"><header class="panel-head"><h2>Training load</h2><span class="panel-meta">Recent effort against your baseline</span></header><div id="status-panel"></div></section>
    <section id="volume" class="panel"><header class="panel-head"><h2>Weekly mileage</h2><span class="panel-meta" id="volume-meta"></span></header><div id="volume-panel" class="bar-chart volume"></div><p id="volume-legend" hidden></p></section>
    <section id="fitness" class="panel"><header class="panel-head"><h2>Fitness</h2><span class="panel-meta">What you could race right now</span></header><div class="carousel"><div id="fitness-cards" class="cards" role="region" aria-label="Fitness estimates" tabindex="0"></div></div></section>
    <details id="recovery" class="panel analysis-detail"><summary><span><h2>Body & recovery</h2><span>Sleep, HRV and resting heart rate</span></span></summary><div class="analysis-content"><div id="recovery-panel"></div><div class="carousel"><div id="wearable-cards" class="cards signals" role="region" aria-label="Recovery signals" tabindex="0"></div></div></div></details>
    <details id="routes" class="panel analysis-detail"><summary><span><h2>Running routes</h2><span>Your recorded routes</span></span></summary><div class="analysis-content"><div id="map"></div></div></details>
  </div>`;