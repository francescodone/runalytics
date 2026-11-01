# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> visual matrix light 320px
- Location: tests/desk.spec.mjs:884:5

# Error details

```
Error: training

expect(received).toBe(expected) // Object.is equality

Expected: true
Received: false
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - link "Skip to workspace" [ref=e3] [cursor=pointer]:
    - /url: "#workspace"
  - banner [ref=e4]:
    - link "Runalytics training" [ref=e5] [cursor=pointer]:
      - /url: "#training"
    - navigation "Workspaces" [ref=e6]:
      - link "Training" [ref=e7] [cursor=pointer]:
        - /url: "#training"
      - link "Activities" [ref=e11] [cursor=pointer]:
        - /url: "#journal"
      - link "Health" [ref=e18] [cursor=pointer]:
        - /url: "#health"
    - generic [ref=e22]:
      - button "Sync activities" [ref=e23] [cursor=pointer]
      - button "Help" [ref=e28] [cursor=pointer]
      - button "Settings" [ref=e33] [cursor=pointer]
  - main [ref=e38]:
    - heading "Training" [level=1] [ref=e40]
    - generic [ref=e41]:
      - generic [ref=e42]:
        - generic [ref=e43]:
          - text: Race preparation / 21K
          - heading "26/10/2026 - 01/11/2026" [level=2] [ref=e44]
        - generic [ref=e45]:
          - button "Schedule race or test" [ref=e46] [cursor=pointer]
          - button "Sync plan to watch" [ref=e49] [cursor=pointer]
      - generic [ref=e55]:
        - navigation "Training horizon" [ref=e56]:
          - link "Week" [ref=e57] [cursor=pointer]:
            - /url: "#training/week"
          - link "Month" [ref=e58] [cursor=pointer]:
            - /url: "#training/month"
          - link "Plan" [ref=e59] [cursor=pointer]:
            - /url: "#training/plan"
        - generic [ref=e60]:
          - button "Previous week" [ref=e61] [cursor=pointer]
          - button "Today" [ref=e66] [cursor=pointer]
          - button "Next week" [ref=e68] [cursor=pointer]
      - generic [ref=e76]:
        - generic "Days this week" [ref=e77]:
          - button "Mon 26/10/2026" [ref=e78] [cursor=pointer]:
            - generic [ref=e79]: Mon
            - strong [ref=e80]: "26"
          - button "Tue 27/10/2026" [ref=e82] [cursor=pointer]:
            - generic [ref=e83]: Tue
            - strong [ref=e84]: "27"
          - button "Wed 28/10/2026" [ref=e86] [cursor=pointer]:
            - generic [ref=e87]: Wed
            - strong [ref=e88]: "28"
          - button "Thu 29/10/2026" [ref=e90] [cursor=pointer]:
            - generic [ref=e91]: Thu
            - strong [ref=e92]: "29"
          - button "Fri 30/10/2026" [ref=e94] [cursor=pointer]:
            - generic [ref=e95]: Fri
            - strong [ref=e96]: "30"
          - button "Sat 31/10/2026" [ref=e98] [cursor=pointer]:
            - generic [ref=e99]: Sat
            - strong [ref=e100]: "31"
          - button "Sun 01/11/2026" [pressed] [ref=e102] [cursor=pointer]:
            - generic [ref=e103]: Sun
            - strong [ref=e104]: "1"
        - generic [ref=e106]:
          - heading "Sun, 01/11/2026" [level=3] [ref=e107]
          - button "Long run Long 19 km 19.0 km" [ref=e108] [cursor=pointer]:
            - generic [ref=e109]: Long run
            - strong [ref=e111]: Long 19 km
            - generic [ref=e112]: 19.0 km
        - generic [ref=e113]:
          - generic [ref=e114]: 4 sessions in this week
          - generic [ref=e115]: 35.0 km
```

# Test source

```ts
  794 |       if (command !== 'activity_review') return result;
  795 |       return {...result, prescribed:!new URLSearchParams(location.search).has('unprescribed'), laps:result.laps.map((lap,index) => index === 1 ? {...lap,avgPaceSKm:null} : index === 3 ? {...lap,bandLoSKm:300,bandHiSKm:280} : lap)};
  796 |     };
  797 |   });
  798 |   await page.goto('/desk.html#journal/1/execution');
  799 |   const missing = page.getByRole('region',{name:'Lap pace chart'}).getByRole('button').nth(1);
  800 |   await expect(missing.locator('.lap-target-band')).toHaveCount(1);
  801 |   await expect(missing.locator('.lap-pace-actual')).toHaveCount(0);
  802 |   await missing.click();
  803 |   await expect(page.locator('.lap-pace-selection')).toContainText('Not recorded');
  804 |   await expect(page.locator('.lap-pace-selection')).toContainText('Not graded');
  805 |   await expect(page.locator('.lap-pace-column').nth(3).locator('.lap-target-band')).toHaveCount(0);
  806 |   expect(await page.locator('.lap-execution').innerHTML()).not.toMatch(/NaN|Infinity/);
  807 |   await page.goto('/desk.html?unprescribed=1#journal/1/execution');
  808 |   await expect(page.getByRole('heading',{name:'Recorded laps',exact:true})).toBeVisible();
  809 |   await expect(page.locator('.lap-target-band, .lap-pace-chart')).toHaveCount(0);
  810 |   await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(0);
  811 |   await expect(page.getByRole('region',{name:'Recorded laps'})).toContainText('Not prescribed');
  812 | });
  813 | 
  814 | for (const width of [320,1440]) {
  815 |   test(`review details remain accessible at ${width}px`, async ({page}) => {
  816 |     await page.setViewportSize({width,height:1000});
  817 |     for(const theme of ['light','dark']) {
  818 |       await page.goto(`/desk.html?theme=${theme}#journal/1/splits`);
  819 |       await expect(page.getByRole('heading',{name:'Kilometre splits'})).toBeVisible();
  820 |       expect(await page.getByRole('region',{name:'Kilometre splits'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
  821 |       await page.getByRole('tab',{name:'Session execution',exact:true}).click();
  822 |       await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
  823 |       expect(await page.getByRole('region',{name:'Recorded laps'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
  824 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'activity review').toEqual([]);
  825 |       expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  826 |       await page.screenshot({path:`test-results/desk-activity-${theme}-${width}.png`,fullPage:true});
  827 |       await page.goto(`/desk.html?proposal=1&theme=${theme}#health/journal`);
  828 |       await page.getByRole('button',{name:'Review changes',exact:true}).click();
  829 |       await expect(page.getByRole('dialog',{name:'Review plan changes'})).toBeVisible();
  830 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'proposal review').toEqual([]);
  831 |       await page.screenshot({path:`test-results/desk-proposal-${theme}-${width}.png`,fullPage:true});
  832 |     }
  833 |   });
  834 | }
  835 | 
  836 | test('ending a plan requires confirmation and keeps recorded history',async ({page})=>{
  837 |   await page.goto('/plan.html');
  838 |   await page.getByRole('button',{name:'End plan',exact:true}).click();
  839 |   expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toEqual([]);
  840 |   await page.getByRole('button',{name:'Keep plan',exact:true}).click();
  841 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  842 |   await expect(page.getByRole('button',{name:'End plan',exact:true})).toBeVisible();
  843 |   await page.getByRole('button',{name:'End plan',exact:true}).click();
  844 |   await page.getByRole('dialog').getByRole('button',{name:'End plan',exact:true}).click();
  845 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  846 |   expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toHaveLength(1);
  847 |   await page.getByRole('link',{name:'Activities',exact:true}).click();
  848 |   await expect(page.locator('.activity-row')).toHaveCount(9);
  849 | });
  850 | 
  851 | test("original entry routes load the independent workspace and retain deep links", async ({page}) => {
  852 |   const loaded=[];
  853 |   page.on('request',request=>loaded.push(request.url()));
  854 |   for(const [url,heading,section] of [
  855 |     ['/', 'Training', 'Week'],
  856 |     ['/index.html', 'Training', 'Week'],
  857 |     ['/plan.html', 'Training', 'Plan'],
  858 |     ['/calendar.html', 'Training', 'Month'],
  859 |     ['/achievements.html', 'Activities', null],
  860 |     ['/achievements.html#performance', 'Health', 'Overview'],
  861 |     ['/achievements.html#recovery', 'Health', 'Overview'],
  862 |     ['/achievements.html#risk', 'Health', 'Overview'],
  863 |     ['/achievements.html#milestones', 'Health', 'Milestones'],
  864 |     ['/injury.html', 'Health', 'Pain journal'],
  865 |     ['/settings.html', 'Settings', null],
  866 |     ['/support.html', 'Help', null],
  867 |   ]) {
  868 |     await page.goto(url);
  869 |     await expect(page.locator('#workspace h1')).toHaveText(heading);
  870 |     if(section) await expect(page.locator('.horizon-switch a[aria-current="page"]')).toHaveText(section);
  871 |     await expect(page.locator('#desk-root')).toBeVisible();
  872 |   }
  873 |   for(const [hash,heading] of [['theme','Appearance'],['zones','Training boundaries'],['tiles','Offline maps'],['publishing','Calendar and devices'],['setup','Watch connection']]) {
  874 |     await page.goto(`/settings.html#${hash}`);
  875 |     await expect(page.getByRole('heading',{name:heading,exact:true})).toBeVisible();
  876 |   }
  877 |   await page.goto('/index.html#journal/1');
  878 |   await expect(page.getByRole('complementary',{name:'Activity review'})).toBeVisible();
  879 |   expect(loaded.filter(url=>/\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  880 | });
  881 | 
  882 | for (const theme of ['light','dark']) {
  883 |   for (const width of [320,390,768,1440]) {
  884 |     test(`visual matrix ${theme} ${width}px`, async ({page}) => {
  885 |       test.setTimeout(90000);
  886 |       const errors=[];
  887 |       page.on('pageerror',error=>errors.push(error.message));
  888 |       await page.setViewportSize({width,height:1000});
  889 |       for (const route of ['training','training/month','training/plan','journal','health','health/journal','health/milestones','settings','help']) {
  890 |         await page.goto(`/desk.html?theme=${theme}#${route}`);
  891 |         await expect(page.locator('#workspace h1')).toBeVisible();
  892 |         await expect(page.locator('.desk-loading')).toHaveCount(0);
  893 |         await expect(page.getByRole('alert')).toHaveCount(0);
> 894 |         expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth),route).toBe(true);
      |                                                                                                   ^ Error: training
  895 |         expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,route).toEqual([]);
  896 |         await page.screenshot({path:`test-results/desk-${route.replace('/','-')}-${theme}-${width}.png`,fullPage:true});
  897 |       }
  898 |       await page.goto(`/desk.html?theme=${theme}#training`);
  899 |       await page.locator(width>800 ? '.week-board .session-tile.movable' : '.day-itinerary .session-tile.movable').first().click();
  900 |       await expect(page.getByRole('complementary',{name:'Session',exact:true})).toBeVisible();
  901 |       if (width<=800) {
  902 |         for(let index=0;index<14;index++) {await page.keyboard.press('Tab');expect(await page.evaluate(()=>Boolean(document.activeElement?.closest('.desk-inspector')))).toBe(true);}
  903 |       }
  904 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'session inspector').toEqual([]);
  905 |       await page.screenshot({path:`test-results/desk-session-${theme}-${width}.png`,fullPage:true});
  906 |       await page.keyboard.press('Escape');
  907 |       await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
  908 |       await expect(page.getByRole('navigation',{name:'Workspaces'})).toBeVisible();
  909 |       expect(errors).toEqual([]);
  910 |     });
  911 |   }
  912 | }
  913 | 
  914 | test("native sync completion refreshes evidence and watch progress is visible", async ({page}) => {
  915 |   await page.goto('/desk.html');
  916 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='plugin:event|listen').length)).toBe(3);
  917 |   await page.evaluate(() => window.__deskEmit('sync://started', null));
  918 |   await expect(page.getByRole('button',{name:'Syncing activities'})).toBeDisabled();
  919 |   const before = await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length);
  920 |   await page.evaluate(() => window.__deskEmit('sync://completed',{ok:true}));
  921 |   await expect(page.getByRole('button',{name:'Sync activities'})).toBeEnabled();
  922 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length)).toBeGreaterThan(before);
  923 |   await page.evaluate(() => {window.__deskHoldPush=true;});
  924 |   await page.getByRole('button',{name:/Send plan to watch|Sync plan to watch/}).click();
  925 |   await page.evaluate(() => window.__deskEmit('plan://sync',{op:'push',percent:45,stage:'Publishing sessions'}));
  926 |   await expect(page.getByRole('progressbar',{name:'Watch publishing progress'})).toHaveAttribute('value','45');
  927 |   await page.evaluate(() => window.__deskResolvePush());
  928 |   await expect(page.getByRole('status')).toContainText('Plan sent to COROS');
  929 | });
  930 | 
  931 | test("pointer drag moves a session to the lane under the pointer and never opens it", async ({page}) => {
  932 |   await page.setViewportSize({width:1440,height:1000});
  933 |   await page.goto('/desk.html');
  934 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  935 |   const tile = page.locator('.week-board .session-tile.movable').first();
  936 |   const lanes = page.locator('.week-board .day-lane');
  937 |   const fromIndex = await lanes.evaluateAll(nodes => nodes.findIndex(node => node.querySelector('.session-tile.movable')));
  938 |   const target = lanes.nth(fromIndex + 1);
  939 |   const a = await tile.boundingBox();
  940 |   const b = await target.boundingBox();
  941 |   await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  942 |   await page.mouse.down();
  943 |   await page.mouse.move(a.x + a.width / 2 + 30, a.y + a.height / 2, {steps: 4});
  944 |   await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, {steps: 8});
  945 |   await expect(target).toHaveAttribute('data-drop-target', '');
  946 |   await page.mouse.up();
  947 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toHaveLength(1);
  948 |   await expect(page.getByRole('status')).toContainText('Moved to');
  949 |   // The click the browser fires after the release must not open the inspector.
  950 |   await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
  951 | });
  952 | 
  953 | test("month view marks finished sessions like the week board does", async ({page}) => {
  954 |   await page.setViewportSize({width:1440,height:1000});
  955 |   await page.goto('/desk.html#training/month');
  956 |   const tiles = page.locator('.month-sessions .session-tile');
  957 |   await expect(tiles.first()).toBeVisible();
  958 |   // The fixture links finished workouts to an activity, so their tiles carry
  959 |   // the same Completed mark as on the week board; the kind label is what the
  960 |   // month cell gives up for space, never the mark.
  961 |   const done = page.locator('.month-sessions .session-tile [aria-label="Completed"]');
  962 |   expect(await done.count()).toBeGreaterThan(0);
  963 |   await expect(done.first()).toBeVisible();
  964 |   await expect(page.locator('.month-sessions .session-kind').first()).toBeHidden();
  965 | });
  966 | 
  967 | test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  968 |   await page.setViewportSize({width:1440,height:1000});
  969 |   await page.goto('/desk.html');
  970 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  971 |   await page.locator('.week-board .session-tile.movable').first().click();
  972 |   const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  973 |   const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  974 |   const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  975 |   await page.getByLabel('Run on',{exact:true}).fill(moved);
  976 |   await page.getByRole('button',{name:'Move session',exact:true}).click();
  977 |   await expect(page.getByRole('status')).toContainText('Moved to');
  978 |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  979 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  980 |   await page.locator('.week-board .session-tile.movable').first().click();
  981 |   await page.getByRole('button',{name:'Skip session',exact:true}).click();
  982 |   await expect(page.getByRole('status')).toContainText('Session skipped');
  983 |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  984 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
  985 | });
  986 | 
  987 | test("matching uses the chosen recorded activity", async ({page}) => {
  988 |   await page.setViewportSize({width:1440,height:1000});
  989 |   await page.goto('/desk.html?unmatched=1');
  990 |   await page.locator('.week-board .session-tile.movable').first().click();
  991 |   await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  992 |   await page.getByRole('button',{name:'Match run',exact:true}).click();
  993 |   await expect(page.getByRole('status')).toContainText('Recorded run matched');
  994 |   expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
```