/* Shared `activity_review` stub for the harnesses.

   Both the dashboard harness and the calendar harness open a recorded run,
   and both must draw the same review — otherwise a fix verified on one page
   can still be broken on the other. The layout below is what the backend's
   `laps::build` returns for a "4 x 1 km tempo" prescription: an open-ended
   warm-up, four graded reps with ungraded recoveries between them, and an
   open-ended cool-down.

   The paces are chosen to cover every verdict the chart can render, because
   each one has its own styling and an untested one is an unstyled one:
   rep 1 too fast, rep 2 in range, rep 3 too slow, rep 4 in range. The warm-up,
   cool-down, and recoveries carry no band at all — only the work is graded,
   and a target that was never set must not be drawn as one. */
(function () {
  // `frac` is the lap's share of the run, so the laps always sum to whatever
  // distance the activity claims and the tiles above the chart agree with
  // the bars below them.
  const LAYOUT = [
    { kind: "easy", label: "Warm-up - easy jog", frac: 0.24, pace: 372, hr: 138 },
    { kind: "work", label: "Fast repetitions - jog between reps only", frac: 0.12, pace: 268, hr: 172, bandLo: 275, bandHi: 285 },
    { kind: "recovery", label: "Recovery", frac: 0.025, pace: 340, hr: 160 },
    { kind: "work", label: "Fast repetitions - jog between reps only", frac: 0.12, pace: 279, hr: 176, bandLo: 275, bandHi: 285 },
    { kind: "recovery", label: "Recovery", frac: 0.025, pace: 348, hr: 162 },
    { kind: "work", label: "Fast repetitions - jog between reps only", frac: 0.12, pace: 291, hr: 178, bandLo: 275, bandHi: 285 },
    { kind: "recovery", label: "Recovery", frac: 0.025, pace: 355, hr: 165 },
    { kind: "work", label: "Fast repetitions - jog between reps only", frac: 0.12, pace: 281, hr: 179, bandLo: 275, bandHi: 285 },
    { kind: "easy", label: "Cool-down - easy jog", frac: 0.205, pace: 402, hr: 134 },
  ];

  // Mirrors `laps::grade`, tolerance included, so the harness cannot show a
  // bar whose colour disagrees with the range drawn over it. No band — no
  // verdict: the easy and recovery laps are never graded.
  function verdictFor(seg, tolerance) {
    if (seg.bandLo == null || seg.bandHi == null) return null;
    if (seg.pace < seg.bandLo - tolerance) return "ahead";
    if (seg.pace > seg.bandHi + tolerance) return "behind";
    return "in";
  }

  /// An open route along the Isar with some wiggle, so the route map, its
  /// start/finish dots and its controls all render.
  function trackFor(activity) {
    const base = Math.floor(Date.parse(activity.startTime) / 1000);
    const track = [];
    for (let i = 0; i <= 80; i++) {
      const t = i / 80;
      track.push({
        ts: base + i * 30,
        lat: 48.1345 + t * 0.038 + Math.sin(t * 9) * 0.0016,
        lon: 11.5768 + t * 0.014 + Math.sin(t * 7) * 0.0022,
        hr: 140 + Math.round(20 * Math.sin(t * 4)),
      });
    }
    return track;
  }

  /// The activity as the review's headline needs it: duration and average pace
  /// derived from the laps themselves. A tile reading 5:50/km above a chart of
  /// 4:28-4:51 reps would look like a rendering bug rather than a stub, and the
  /// whole point of the chart is that the two agree.
  function summarised(activity, laps) {
    const durationS = laps.reduce((sum, l) => sum + l.durationS, 0);
    const distanceM = laps.reduce((sum, l) => sum + l.meters, 0);
    return {
      ...activity,
      durationS,
      distanceM,
      avgPaceSKm: distanceM > 0 ? Math.round((durationS / distanceM) * 1000) : null,
      hasTrack: true,
    };
  }

  window.__reviewFor = function (activity, workoutId, tolerance) {
    const tol = tolerance ?? 4;
    const total = activity.distanceM;
    let startM = 0;
    const laps = LAYOUT.map((seg, i) => {
      const meters = Math.round(seg.frac * total);
      const lap = {
        index: i + 1,
        label: seg.label,
        kind: seg.kind,
        startM,
        meters,
        durationS: Math.round((meters / 1000) * seg.pace),
        avgPaceSKm: seg.pace,
        avgHr: seg.hr,
        bandLoSKm: seg.bandLo ?? null,
        bandHiSKm: seg.bandHi ?? null,
        verdict: verdictFor(seg, tol),
      };
      startM += meters;
      return lap;
    });

    // Kilometre splits, cut by distance rather than by the plan, walked over
    // the same laps so the table and the chart can never tell two stories.
    // The last window is a part-kilometre, exactly like the backend's.
    const splits = [];
    let pi = 0;
    let piUsed = 0;
    let kmMetres = 0;
    let kmDur = 0;
    let kmHr = 0;
    let prev = null;
    while (pi < laps.length || kmMetres > 0) {
      if (pi < laps.length) {
        const lap = laps[pi];
        const seg = LAYOUT[pi];
        const take = Math.min(lap.meters - piUsed, 1000 - kmMetres);
        kmDur += (take / 1000) * seg.pace;
        // Metre-weighted, not lap-weighted: a kilometre that spans a rep and a
        // recovery averages the two rates by how much of each it contains.
        kmHr += take * seg.hr;
        kmMetres += take;
        piUsed += take;
        if (piUsed >= lap.meters) {
          pi++;
          piUsed = 0;
        }
      }
      if (kmMetres >= 1000 || (pi >= laps.length && kmMetres > 0)) {
        const avg = kmDur / (kmMetres / 1000);
        const seg = LAYOUT[Math.min(pi, laps.length - 1)];
        splits.push({
          km: splits.length + 1,
          meters: kmMetres,
          durationS: Math.round(kmDur),
          avgPaceSKm: Math.round(avg),
          avgHr: Math.round(kmHr / kmMetres),
          deltaPrevSKm: prev == null ? null : Math.round(avg - prev),
          bandLoSKm: seg.bandLo ?? null,
          bandHiSKm: seg.bandHi ?? null,
        });
        prev = avg;
        kmMetres = 0;
        kmDur = 0;
        kmHr = 0;
      }
    }

    return {
      activity: summarised(activity, laps),
      hasTrack: true,
      track: trackFor(activity),
      workoutId: workoutId ?? null,
      prescribed: true,
      laps,
      splits,
    };
  };

  /// The fallback the backend uses when no session was planned for a run: the
  /// kilometres become the laps, with no target caps anywhere. Worth having in
  /// the harness because the chart's legend and the empty band both read
  /// differently here, and it is the state most runs on the dashboard are in.
  window.__kmReviewFor = function (activity) {
    const track = trackFor(activity);
    const splits = [];
    const total = activity.distanceM;
    const n = Math.ceil(total / 1000);
    let prev = null;
    for (let i = 0; i < n; i++) {
      const meters = Math.min(1000, total - i * 1000);
      // A gentle negative split across the run, so the delta column shows both
      // signs rather than a wall of one colour.
      const pace = (activity.avgPaceSKm ?? 340) + 12 - (i / Math.max(1, n - 1)) * 24;
      splits.push({
        km: i + 1,
        meters,
        durationS: Math.round((meters / 1000) * pace),
        avgPaceSKm: Math.round(pace),
        avgHr: activity.avgHr == null ? null : activity.avgHr + 6 - i,
        deltaPrevSKm: prev == null ? null : Math.round(pace - prev),
        bandLoSKm: null,
        bandHiSKm: null,
        openEnded: false,
      });
      prev = pace;
    }
    const laps = splits.map((s) => ({
      index: s.km,
      label: `${s.km} km`,
      kind: "kilometre",
      startM: (s.km - 1) * 1000,
      meters: s.meters,
      durationS: s.durationS,
      avgPaceSKm: s.avgPaceSKm,
      avgHr: s.avgHr,
      bandLoSKm: null,
      bandHiSKm: null,
      openEnded: false,
      verdict: null,
    }));
    return {
      activity: summarised(activity, laps),
      hasTrack: true,
      track,
      workoutId: null,
      prescribed: false,
      laps,
      splits,
    };
  };
})();
