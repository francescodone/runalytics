import { readFileSync } from "node:fs";

const dashboardFixture = readFileSync(new URL("../__harness.html", import.meta.url), "utf8").match(/<script>([\s\S]*?)<\/script>/)[1];
const reviewFixture = readFileSync(new URL("../__review_stub.js", import.meta.url), "utf8");

export const deskFixture = `(function () {
  ${reviewFixture}
  ${dashboardFixture}
  const deskOriginal = window.__TAURI_INTERNALS__.invoke;
  let deskCurrent;
  let deskSettings;
  let deskPain = [];
  let deskMessages = [];
  let deskPublishing = {enabled:false, uploadUrl:'https://example.test/calendar.ics', subscriptionUrl:'https://example.test/calendar.ics', username:'synthetic', hasPassword:true, lastPublished:'2026-10-29T12:00:00Z', lastError:null};
  let deskSync = {config:{enabled:true,gistId:'synthetic-gist',hasToken:true,canPublish:false,lastPushed:'2026-10-29T12:00:00Z',lastPulled:null,installedRev:7,lastError:null},accountLinked:true,code:'synthetic-account-channel'};
  let deskKey = true;
  let deskConnected = true;
  let deskStrava = {configured:false,clientId:'',connected:false,athleteId:null,enabled:false,enabledAt:null,updated:0,alreadyPresent:0,pending:0,needsReview:0,lastMessage:null,lastAttemptAt:null};
  // Stands in for the backend's original_json snapshot: what a session was
  // before a benchmark overwrote it ('created' when the plan had a rest day).
  const deskBenchmarks = new Map();
  const callbacks = new Map();
  const listeners = new Map();
  let callbackId = 0;
  // The first three done plan sessions stand in for matched runs: the same
  // pairing the backend derives from calendar_completions, so the journal can
  // name a matched run after its session and the review can show its plan.
  const deskMatched = async () => { const pair = await window.__TAURI_INTERNALS__.invoke('current_plan'); return (pair?.[1] ?? []).filter(workout => workout.done).slice(0, 3); };
  window.__TAURI_INTERNALS__.transformCallback = callback => { callbacks.set(++callbackId, callback); return callbackId; };
  window.__TAURI_INTERNALS__.unregisterCallback = identifier => callbacks.delete(identifier);
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
  window.__deskEmit = (event, payload) => { const listener = listeners.get(event); callbacks.get(listener)?.({event,id:listener,payload}); };
  let proposal = new URLSearchParams(location.search).has('proposal') ? {id:'synthetic-proposal',source:'pain',status:'pending',summary:'Reduce the next quality session',reason:'Synthetic recovery adjustment',createdAt:'2026-10-30',edits:[{workoutId:'synthetic-workout',date:'2026-11-01',oldTitle:'Intervals',oldDistanceM:8000,title:'Easy run',distanceM:4000,kind:'easy'}]} : null;
  if (new URLSearchParams(location.search).has('pacingProposal')) proposal = {id:'synthetic-pacing',source:'pacing',status:'pending',summary:'Sharpen quality pace by 5 s/km and review future session distances',reason:'Three completed quality sessions consistently faster than their own targets; recovery and load support progression.',createdAt:'2026-10-31',planQualityOffsetSecKm:-5,planQualityOffsetFrom:'2026-11-01',edits:[{workoutId:'synthetic-workout',date:'2026-11-02',oldTitle:'Tempo',oldDistanceM:10000,title:'Tempo',distanceM:10300,kind:'tempo',paceLoSKm:275,paceHiSKm:285}]};
  window.__deskCalls = [];
  window.__deskFail = '';
  window.__deskHoldPush = new URLSearchParams(location.search).has('holdPush');
  window.__TAURI_INTERNALS__.invoke = async (cmd, args = {}) => {
    window.__deskCalls.push({command:cmd, args});
    if (cmd === window.__deskFail) throw new Error('Unable to save right now');
    if (cmd === 'strava_status') return {...deskStrava};
    if (cmd === 'strava_configure') { deskStrava={...deskStrava,configured:true,clientId:args.clientId}; return {...deskStrava}; }
    if (cmd === 'strava_connect') { if (window.__deskHoldStrava) await new Promise((resolve,reject) => { window.__deskCancelStrava=() => reject(new Error('Strava sign-in was cancelled.')); }); deskStrava={...deskStrava,connected:true,athleteId:42,enabled:false,pending:2}; return {...deskStrava}; }
    if (cmd === 'strava_cancel_connect') { window.__deskCancelStrava?.(); return null; }
    if (cmd === 'strava_set_enabled') { deskStrava={...deskStrava,enabled:args.enabled,enabledAt:deskStrava.enabledAt ?? Math.floor(Date.now()/1000)}; return {...deskStrava}; }
    if (cmd === 'strava_disconnect') { deskStrava={...deskStrava,connected:false,enabled:false,athleteId:null,...(args.forget ? {configured:false,clientId:''} : {})}; return {...deskStrava}; }
    if (cmd === 'strava_retry') { deskStrava={...deskStrava,updated:1,alreadyPresent:1,pending:0,lastMessage:'Strava: 1 copied, 1 already present, 0 pending, 0 need attention.'}; return {...deskStrava}; }
    if (cmd === 'current_plan') {
      if (deskCurrent !== undefined) return deskCurrent;
      if (new URLSearchParams(location.search).has('noPlan')) return null;
      const pair = await deskOriginal(cmd, args);
      deskCurrent = [{...pair[0],targetDate:pair[0].targetDate.slice(0,10)}, pair[1].map(workout => ({...workout, plannedDate:workout.date, status:'planned', movedAt:null}))];
      // ?noPush=1 stands in for a plan that has never reached the watch —
      // the state in which the Training page pushes on its own.
      if (new URLSearchParams(location.search).has('noPush')) deskCurrent[0] = {...deskCurrent[0], status:'active', pushedId:null};
      return deskCurrent;
    }
    if (cmd === 'get_settings') return deskSettings ?? {...await deskOriginal(cmd, args), theme:new URLSearchParams(location.search).get('theme') ?? 'light', palette:new URLSearchParams(location.search).get('palette') ?? 'reference',llmEnabled:true,llmEndpoint:'https://example.test/v1',llmModel:'test-model'};
    if (cmd === 'save_settings') { deskSettings = args.settings; return null; }
    if (cmd === 'provider_status') return [{provider:'coros',connected:deskConnected,lastSync:new Date().toISOString(),detail:'Synthetic account'}];
    if (cmd === 'coros_signin_notice') return null;
    if (cmd === 'disconnect_provider') { deskConnected=false; return null; }
    if (cmd === 'connect_provider') { deskConnected=true; return 'connected'; }
    if (cmd === 'check_provider_api') return ['synthetic-list-activities'];
    if (cmd === 'plan_defaults') return {currentWeeklyKm:32, thresholdSecKm:280, predict5kS:1200, predict10kS:2500, predictHalfS:5400, predictMarathonS:11400, thisWeekKm:12,totalKm:300,weeksObserved:4,lastSync:new Date().toISOString(),activePain:null};
    if (cmd === 'create_plan') {
      deskCurrent = [{id:'synthetic-plan',...args.request,status:'active',createdAt:new Date().toISOString(),pushedId:null,baselineWeeklyKm:args.request.currentWeeklyKm,qualityOffsetSecKm:0,qualityOffsetFrom:null},[]];
      return deskCurrent;
    }
    if (cmd === 'run_scores') { const done = await deskMatched(); return done.map((workout, index) => ({ activityId: index + 1, workoutId: workout.id, load: 120 + index * 15, risk: 40 + index, riskDelta: index % 2 ? 1.4 : -0.8 })).concat([{ activityId: 4, workoutId: null, load: 96, risk: 38, riskDelta: -1.2 }]); }
    if (cmd === 'matched_sessions') { const done = await deskMatched(); return done.map((workout, index) => ({ activityId: index + 1, title: workout.title, purpose: 'Build endurance at conversational pace so the body adapts without accumulating fatigue.', notes: 'Made with Runalytics - a virtual personal vibecoded coach \\n\\n' + workout.title + '\\n\\nPlan:\\n  2 km easy\\n\\nWhy: Build endurance at conversational pace.' })); }
    if (cmd === 'calendar_events') {
      const pair = await window.__TAURI_INTERNALS__.invoke('current_plan');
      const events = (pair?.[1] ?? []).filter(workout => workout.date >= args.from && workout.date <= args.to).map(workout => ({...workout,workoutId:workout.id,activityId:workout.activityId ?? (workout.done ? 1 : null),notes:'Warm-up\\n2 km easy\\n\\nRepeat 4 times:\\n  1 km at 4:40/km\\n  60 seconds walking rest\\n\\nCool-down\\n1 km easy',adjusted:null,moved:workout.date!==workout.plannedDate,missed:false,start:workout.date,end:workout.date,allDay:true}));
      if (new URLSearchParams(location.search).has('unmatched')) events.push({id:'unmatched',workoutId:null,activityId:99,date:args.from,kind:'completed',title:'Recorded run',distanceM:6000,durationMin:30,notes:'',adjusted:null,moved:false,missed:false,start:args.from,end:args.from,allDay:false});
      return events;
    }
    if (cmd === 'move_workout') { deskCurrent[1].find(workout => workout.id === args.workoutId).date = args.date; return null; }
    if (cmd === 'schedule_benchmark') {
      const existing = deskCurrent[1].find(workout => workout.date === args.date);
      if (existing && !deskBenchmarks.has(existing.id)) deskBenchmarks.set(existing.id, {...existing});
      const benchmark = {...existing,id:existing?.id ?? 'synthetic-benchmark',planId:deskCurrent[0].id,date:args.date,plannedDate:existing?.plannedDate ?? args.date,kind:'race',title:'Benchmark: '+args.name,distanceM:args.distanceM,done:false,status:'planned'};
      if (!existing) deskBenchmarks.set(benchmark.id, 'created');
      deskCurrent[1] = deskCurrent[1].filter(workout => workout.id !== benchmark.id).concat(benchmark);
      return null;
    }
    if (cmd === 'set_workout_skipped') { deskCurrent[1].find(workout => workout.id === args.workoutId).status = args.skipped ? 'skipped' : 'planned'; return null; }
    if (cmd === 'abort_plan') { deskCurrent = null; return null; }
    if (cmd === 'clear_local_plan_data') { deskCurrent = null; return null; }
    if (cmd === 'push_plan_to_provider') { if (window.__deskHoldPush) await new Promise(resolve => { window.__deskResolvePush=resolve; }); deskCurrent[0] = {...deskCurrent[0], pushedId:'watch-plan', status:'pushed'}; return 'published'; }
    if (cmd === 'match_calendar_activity') { const workout=deskCurrent[1].find(workout=>workout.id===args.workoutId);workout.activityId=args.activityId;workout.done=true;return null; }
    if (cmd === 'week_deviations') { const sunday = new Date(args.weekStart + 'T12:00:00'); sunday.setDate(sunday.getDate() + 6); const key = d => d.toISOString().slice(0,10); return (deskCurrent?.[1] ?? []).filter(workout=>workout.date>=args.weekStart && workout.date<=key(sunday) && (workout.date!==workout.plannedDate || deskBenchmarks.has(workout.id))).map(workout=>{const original=deskBenchmarks.get(workout.id);const pending=!workout.done && workout.status!=='skipped';const restoresDate=workout.date!==workout.plannedDate && pending && workout.plannedDate>=new Date().toISOString().slice(0,10);return {workoutId:workout.id,title:workout.title,from:workout.date,to:workout.plannedDate,restorable:restoresDate || (pending && original!==undefined),isToday:workout.plannedDate===new Date().toISOString().slice(0,10),restoresDate,restoresContent:pending && original!==undefined && original!=='created',removes:original==='created',};});}
    if (cmd === 'plan_week_progress') { const pair = await window.__TAURI_INTERNALS__.invoke('current_plan'); const weeks = new Map(); for (const workout of pair?.[1] ?? []) { const monday = new Date(workout.date + 'T12:00:00'); monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7)); const key = monday.getFullYear() + '-' + String(monday.getMonth()+1).padStart(2,'0') + '-' + String(monday.getDate()).padStart(2,'0'); const entry = weeks.get(key) ?? {weekStart:key,plannedM:0,ranM:0}; entry.plannedM += workout.distanceM ?? 0; if (workout.done) entry.ranM += workout.distanceM ?? 0; weeks.set(key, entry); } return [...weeks.values()].sort((a,b)=>a.weekStart.localeCompare(b.weekStart)); }
    if (cmd === 'reset_week') { const sunday = new Date(args.weekStart + 'T12:00:00'); sunday.setDate(sunday.getDate() + 6); const key = d => d.toISOString().slice(0,10); let count = 0; for (const workout of deskCurrent[1].filter(workout=>workout.date>=args.weekStart && workout.date<=key(sunday))) {
      const original = deskBenchmarks.get(workout.id);
      const pending = !workout.done && workout.status !== 'skipped';
      if (original === 'created' && pending) { deskCurrent[1] = deskCurrent[1].filter(entry=>entry.id!==workout.id); deskBenchmarks.delete(workout.id); count += 1; continue; }
      if (original && pending) { Object.assign(workout, original, {date: workout.date, plannedDate: workout.plannedDate}); deskBenchmarks.delete(workout.id); count += 1; continue; }
      if (workout.date !== workout.plannedDate && pending && workout.plannedDate >= new Date().toISOString().slice(0,10)) { workout.date = workout.plannedDate; workout.movedAt = null; count += 1; }
    } return count; }
    if (cmd === 'plan_sync_status') return deskSync;
    if (cmd === 'plan_sync_refresh') {
      if (window.__deskHoldRefresh) await new Promise(resolve => { window.__deskReleaseRefresh=resolve; });
      deskSync.config.lastChecked=new Date().toISOString();
      window.__deskEmit('plan://checked',null);
      if (window.__deskRemotePlan === undefined) return false;
      deskCurrent=structuredClone(window.__deskRemotePlan);
      delete window.__deskRemotePlan;
      deskSync.config.lastPulled=new Date().toISOString();
      window.__deskEmit('plan://updated',null);
      return true;
    }
    if (cmd === 'plan_sync_configure') { deskSync.config.enabled = args.config.enabled; if (args.token) deskSync.config.hasToken=true; return deskSync; }
    if (cmd === 'plan_sync_check') return {status:'upToDate',planName:'Synthetic plan',workouts:33,syncedAt:Math.floor(Date.now()/1000)};
    if (cmd === 'plan_sync_push') return deskSync.config;
    if (cmd === 'plan_sync_pull') return {installed:!!args.force,planName:'Synthetic plan',workouts:33,conflict:new URLSearchParams(location.search).has('syncConflict') && !args.force ? 'Different synthetic plan on this device.' : null};
    if (cmd === 'calendar_publishing') return deskPublishing;
    if (cmd === 'save_calendar_publishing') { deskPublishing={...deskPublishing,...args.config,hasPassword:args.password===null ? deskPublishing.hasPassword : args.password!==''}; return deskPublishing; }
    if (cmd === 'ics_path') return '/synthetic/calendar.ics';
    if (cmd === 'publish_calendar') { deskPublishing.lastPublished=new Date().toISOString(); return 'https://example.test/training.ics'; }
    if (cmd === 'llm_status') { const settings=await window.__TAURI_INTERNALS__.invoke('get_settings'); return {configured:settings.llmEnabled,hasKey:deskKey,endpoint:settings.llmEndpoint,model:settings.llmModel}; }
    if (cmd === 'clear_llm_api_key') { deskKey=false; return null; }
    if (cmd === 'set_llm_api_key') { deskKey=true; return null; }
    if (cmd === 'pain_entries') return deskPain;
    if (cmd === 'save_pain_entry') { const entry={id:'synthetic-pain',bodyPart:args.bodyPart,intensity:args.intensity,startedOn:args.startedOn,notes:args.notes,createdAt:new Date().toISOString(),resolvedOn:null,followupAt:null};deskPain.push(entry);return [entry,null]; }
    if (cmd === 'resolve_pain') { deskPain.find(entry=>entry.id===args.id).resolvedOn=args.resolvedOn; return null; }
    if (cmd === 'mark_pain_followup') { deskPain.find(entry=>entry.id===args.id).followupAt=new Date().toISOString(); return null; }
    if (cmd === 'delete_pain_entry') { deskPain=deskPain.filter(entry=>entry.id!==args.id);return null; }
    if (cmd === 'pain_messages') return deskMessages;
    if (cmd === 'clear_pain_messages') { deskMessages=[];return null; }
    if (cmd === 'ask_assistant') { const answer={id:'reply',entryId:args.entryId,bodyPart:'knee.left',role:'assistant',content:'Consider reducing the session and discussing persistent pain with a clinician.',ts:new Date().toISOString()};deskMessages.push({id:'question',entryId:args.entryId,bodyPart:'knee.left',role:'user',content:args.question,ts:new Date().toISOString()},answer);return answer; }
    if (cmd === 'pending_proposal') return proposal?.source==='pain' && proposal?.status==='pending' ? proposal : null;
    if (cmd === 'pending_pacing_proposal') return proposal?.source==='pacing' && proposal?.status==='pending' ? proposal : null;
    if (cmd === 'proposals') return proposal ? [proposal] : [];
    if (cmd === 'apply_proposal') {
      if(proposal.source==='pacing') {
        const pair=await window.__TAURI_INTERNALS__.invoke('current_plan');
        deskCurrent=[{...pair[0],qualityOffsetSecKm:proposal.planQualityOffsetSecKm,qualityOffsetFrom:proposal.planQualityOffsetFrom},pair[1]];
      }
      proposal.status='applied'; return null;
    }
    if (cmd === 'dismiss_proposal') { proposal.status='dismissed'; return null; }
    if (['pain_days','pain_by_region'].includes(cmd)) return [];
    if (cmd === 'pace_insights') {
      if (new URLSearchParams(location.search).has('noPlan')) return null;
      const day = offset => { const date=new Date();date.setDate(date.getDate()+offset);return date.toLocaleDateString('en-CA'); };
      const rep = (index, actualSKm, bandLoSKm=275, bandHiSKm=285) => ({index,label:'Work rep',actualSKm,bandLoSKm,bandHiSKm,relativePace:((bandLoSKm+bandHiSKm)/2-actualSKm)/Math.max((bandHiSKm-bandLoSKm)/2,1)});
      const session = (index, reps, kind='intervals') => ({workoutId:'insight-'+index,activityId:1,date:day(-30+index*5),title:kind==='tempo'?'Continuous tempo':kind==='longrun'?'Race-pace long run':'Interval session '+index,kind,outdated:index===0,reps});
      return {speed:[session(0,[rep(2,268),rep(4,291)]),session(1,[rep(2,279),rep(4,281)]),session(2,[rep(2,280)],'tempo'),session(3,[rep(2,276),rep(4,300)]),session(4,[rep(2,265),rep(4,279)])],long:[session(1,[rep(2,322,315,325),rep(4,328,320,330)],'longrun')],nextSpeed:{id:'next-speed',date:day(3),title:'Tempo 5 km',distanceM:8000},nextLong:{id:'next-long',date:day(5),title:'Long run 13 km',distanceM:13000}};
    }
    if (['plan_receipt','pending_proposal','pending_pacing_proposal','pacing_report','session_review'].includes(cmd)) return null;
    if (cmd === 'plugin:event|listen') { listeners.set(args.event,args.handler); return args.handler; }
    if (cmd === 'plugin:event|unlisten') { listeners.delete(args.event); return null; }
    return deskOriginal(cmd,args);
  };
})();`;

export async function installDeskFixture(page) {
  await page.addInitScript({ content: deskFixture });
}