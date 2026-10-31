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
  const callbacks = new Map();
  const listeners = new Map();
  let callbackId = 0;
  window.__TAURI_INTERNALS__.transformCallback = callback => { callbacks.set(++callbackId, callback); return callbackId; };
  window.__TAURI_INTERNALS__.unregisterCallback = identifier => callbacks.delete(identifier);
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
  window.__deskEmit = (event, payload) => { const listener = listeners.get(event); callbacks.get(listener)?.({event,id:listener,payload}); };
  let proposal = new URLSearchParams(location.search).has('proposal') ? {id:'synthetic-proposal',source:'pain',status:'pending',summary:'Reduce the next quality session',reason:'Synthetic recovery adjustment',createdAt:'2026-10-30',edits:[{workoutId:'synthetic-workout',date:'2026-11-01',oldTitle:'Intervals',oldDistanceM:8000,title:'Easy run',distanceM:4000,kind:'easy'}]} : null;
  if (new URLSearchParams(location.search).has('pacingProposal')) proposal = {id:'synthetic-pacing',source:'pacing',status:'pending',summary:'Sharpen quality pace by 5 s/km and review future session distances',reason:'Three completed quality sessions consistently faster than their own targets; recovery and load support progression.',createdAt:'2026-10-31',planQualityOffsetSecKm:-5,planQualityOffsetFrom:'2026-11-01',edits:[{workoutId:'synthetic-workout',date:'2026-11-02',oldTitle:'Tempo',oldDistanceM:10000,title:'Tempo',distanceM:10300,kind:'tempo',paceLoSKm:275,paceHiSKm:285}]};
  window.__deskCalls = [];
  window.__deskFail = '';
  window.__TAURI_INTERNALS__.invoke = async (cmd, args = {}) => {
    window.__deskCalls.push({command:cmd, args});
    if (cmd === window.__deskFail) throw new Error('Unable to save right now');
    if (cmd === 'current_plan') {
      if (deskCurrent !== undefined) return deskCurrent;
      if (new URLSearchParams(location.search).has('noPlan')) return null;
      const pair = await deskOriginal(cmd, args);
      deskCurrent = [pair[0], pair[1].map(workout => ({...workout, plannedDate:workout.date, status:'planned', movedAt:null}))];
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
    if (cmd === 'calendar_events') {
      const pair = await window.__TAURI_INTERNALS__.invoke('current_plan');
      const events = (pair?.[1] ?? []).filter(workout => workout.date >= args.from && workout.date <= args.to).map(workout => ({...workout,workoutId:workout.id,activityId:workout.activityId ?? (workout.done ? 1 : null),notes:'Warm-up\\n2 km easy\\n\\nRepeat 4 times:\\n  1 km at 4:40/km\\n  60 seconds walking rest\\n\\nCool-down\\n1 km easy',adjusted:null,moved:workout.date!==workout.plannedDate,missed:false,start:workout.date,end:workout.date,allDay:true}));
      if (new URLSearchParams(location.search).has('unmatched')) events.push({id:'unmatched',workoutId:null,activityId:99,date:args.from,kind:'completed',title:'Recorded run',distanceM:6000,durationMin:30,notes:'',adjusted:null,moved:false,missed:false,start:args.from,end:args.from,allDay:false});
      return events;
    }
    if (cmd === 'move_workout') { deskCurrent[1].find(workout => workout.id === args.workoutId).date = args.date; return null; }
    if (cmd === 'set_workout_skipped') { deskCurrent[1].find(workout => workout.id === args.workoutId).status = args.skipped ? 'skipped' : 'planned'; return null; }
    if (cmd === 'abort_plan') { deskCurrent = null; return null; }
    if (cmd === 'push_plan_to_provider') { if (window.__deskHoldPush) await new Promise(resolve => { window.__deskResolvePush=resolve; }); deskCurrent[0].pushedId = 'watch-plan'; return 'published'; }
    if (cmd === 'match_calendar_activity') { const workout=deskCurrent[1].find(workout=>workout.id===args.workoutId);workout.activityId=args.activityId;workout.done=true;return null; }
    if (cmd === 'week_deviations') return (deskCurrent?.[1] ?? []).filter(workout=>workout.date!==workout.plannedDate).map(workout=>({workoutId:workout.id,title:workout.title,from:workout.date,to:workout.plannedDate,restorable:true}));
    if (cmd === 'reset_week') { deskCurrent[1].forEach(workout=>{workout.date=workout.plannedDate;});return null; }
    if (cmd === 'plan_sync_status') return deskSync;
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