# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> training layout and accessibility at 320px
- Location: tests/desk.spec.mjs:1125:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

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
      - link "Journal" [ref=e11] [cursor=pointer]:
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
          - button "Update watch" [ref=e49] [cursor=pointer]
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
          - button "Long run Completed Long 19 km 19.0 km" [ref=e108] [cursor=pointer]:
            - generic [ref=e109]:
              - generic [ref=e110]: Long run
              - img "Completed" [ref=e111]
            - strong [ref=e113]: Long 19 km
            - generic [ref=e114]: 19.0 km
        - generic [ref=e115]:
          - generic [ref=e116]: 4 sessions in this week
          - generic [ref=e117]: 35.0 km
```

# Test source

```ts
  1029 |     const before=await pixels();
  1030 |     await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  1031 |     await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
  1032 |     await page.getByRole('button',{name:'Fit training area',exact:true}).click();
  1033 |     expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  1034 |     await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  1035 |   });
  1036 | }
  1037 | 
  1038 | test("independent shell loads only new presentation", async ({ page }) => {
  1039 |   const loaded = [];
  1040 |   page.on("request", request => loaded.push(request.url()));
  1041 |   await page.goto("/desk.html");
  1042 |   await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  1043 |   await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  1044 |   await expect(page.locator(".day-itinerary")).toBeVisible();
  1045 |   expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1046 | });
  1047 | 
  1048 | test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  1049 |   await page.setViewportSize({width:390, height:844});
  1050 |   await page.emulateMedia({reducedMotion:"no-preference"});
  1051 |   await page.goto("/desk.html#health");
  1052 |   await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  1053 |   await page.locator(".mobile-date-ribbon button").first().click();
  1054 |   const session = page.locator(".day-itinerary .session-tile").first();
  1055 |   await session.click();
  1056 |   await expect(page.getByRole("complementary", { name: "Session", exact: true })).toBeVisible();
  1057 |   await expect.poll(async () => {
  1058 |     const bounds = await page.locator(".desk-inspector").boundingBox();
  1059 |     return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  1060 |   }).toEqual({x:0,y:0,width:390,height:844});
  1061 |   await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  1062 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1063 |   await expect(session).toBeFocused();
  1064 | });
  1065 | 
  1066 | for (const width of [390, 1440]) {
  1067 |   test(`benchmark scheduling submits parkrun and custom test contracts at ${width}px`, async ({ page }) => {
  1068 |     await page.setViewportSize({ width, height: 900 });
  1069 |     await page.goto("/desk.html#training/week");
  1070 |     const open = page.getByRole("button", { name: "Schedule race or test", exact: true });
  1071 |     await open.click();
  1072 |     const dialog = page.getByRole("dialog", { name: "Schedule race or test", exact: true });
  1073 |     await expect(dialog.getByLabel("Distance (km)")).toHaveValue("5");
  1074 |     await expect(dialog.getByLabel("Distance (km)")).toBeDisabled();
  1075 |     const date = await dialog.getByLabel("Date", { exact: true }).inputValue();
  1076 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1077 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1078 |     await expect(dialog).toHaveCount(0);
  1079 |     expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "schedule_benchmark").args)).toEqual({ date, name: "parkrun", distanceM: 5000 });
  1080 |     await expect(page.locator(".day-itinerary")).toContainText("Benchmark: parkrun");
  1081 |     await open.click();
  1082 |     await dialog.getByRole("combobox", { name: "Event", exact: true }).selectOption("trial");
  1083 |     await dialog.getByLabel("Event name", { exact: true }).fill("3 km fitness test");
  1084 |     await dialog.getByLabel("Distance (km)").fill("3");
  1085 |     await page.evaluate(() => { window.__deskFail = "schedule_benchmark"; });
  1086 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1087 |     await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
  1088 |     await expect(dialog.getByLabel("Event name", { exact: true })).toHaveValue("3 km fitness test");
  1089 |     await page.screenshot({ path: `test-results/benchmark-${width}.png`, animations: "disabled" });
  1090 |     await page.evaluate(() => { window.__deskFail = ""; });
  1091 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1092 |     await expect(dialog).toHaveCount(0);
  1093 |     expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "schedule_benchmark").pop().args)).toMatchObject({ name: "3 km fitness test", distanceM: 3000 });
  1094 |   });
  1095 | }
  1096 | 
  1097 | test("plan creation submits the established contract", async ({ page }) => {
  1098 |   await page.goto("/desk.html?noPlan=1");
  1099 |   await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1100 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1101 |   await expect(page.getByLabel("Threshold pace (min/km)")).toHaveValue("4:40");
  1102 |   await page.getByLabel("Weekly distance (km)").fill("40");
  1103 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1104 |   await page.getByRole("button", { name: "Build plan", exact: true }).click();
  1105 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1106 |   const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
  1107 |   expect(request).toMatchObject({kind:"race",distance:"21K",thresholdSecKm:280,weeklyFrequency:4,currentWeeklyKm:40});
  1108 |   await expect(page.locator(".plan-basis")).toContainText("40.0 km/week");
  1109 | });
  1110 | 
  1111 | test("failed plan creation keeps the reviewed draft", async ({ page }) => {
  1112 |   await page.goto("/desk.html?noPlan=1");
  1113 |   await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1114 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1115 |   await page.getByLabel("Weekly distance (km)").fill("37");
  1116 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1117 |   await page.evaluate(() => { window.__deskFail = "create_plan"; });
  1118 |   await page.getByRole("button", { name: "Build plan", exact: true }).click();
  1119 |   await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  1120 |   await expect(page.locator(".composer-summary")).toContainText("37 km/week");
  1121 |   await expect(page.getByRole("button", { name: "Build plan", exact: true })).toBeEnabled();
  1122 | });
  1123 | 
  1124 | for (const width of [320, 390, 768, 1440]) {
  1125 |   test(`training layout and accessibility at ${width}px`, async ({ page }) => {
  1126 |     await page.setViewportSize({ width, height: 1000 });
  1127 |     await page.goto("/desk.html");
  1128 |     await expect(page.locator(width > 800 ? ".week-board" : ".day-itinerary")).toBeVisible();
> 1129 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
       |                                                                                           ^ Error: expect(received).toBe(expected) // Object.is equality
  1130 |     expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  1131 |     await page.screenshot({path:`test-results/desk-training-${width}.png`,fullPage:true});
  1132 |   });
  1133 | }
  1134 | 
  1135 | test("Journal combines filters and opens recorded splits", async ({ page }) => {
  1136 |   await page.goto("/desk.html#journal");
  1137 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1138 |   await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  1139 |   await expect(page.locator(".activity-row")).toHaveCount(6);
  1140 |   await page.getByLabel("Search activities").fill("not-a-run");
  1141 |   await expect(page.getByRole("heading", { name: "No matching activities" })).toBeVisible();
  1142 |   await page.getByRole("button", { name: "Reset filters" }).click();
  1143 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1144 |   await page.locator(".activity-row").first().click();
  1145 |   await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  1146 |   await page.getByRole("tab", { name: "Splits", exact: true }).click();
  1147 |   await expect(page.getByRole("heading", { name: "Kilometre splits" })).toBeVisible();
  1148 |   await expect(page.getByRole("columnheader", { name: "vs previous km" })).toBeVisible();
  1149 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1150 |   await expect(page.locator(".activity-row").first()).toBeFocused();
  1151 | });
  1152 | 
  1153 | test("pain logging retains failure drafts and episode conversations survive refresh", async ({ page }) => {
  1154 |   await page.goto("/desk.html#health/journal");
  1155 |   await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
  1156 |   await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
  1157 |   await page.getByRole("textbox", { name: "Notes", exact: true }).fill("Synthetic test: soreness after running");
  1158 |   await page.evaluate(() => { window.__deskFail = "save_pain_entry"; });
  1159 |   await page.getByRole("button", { name: "Save pain entry" }).click();
  1160 |   await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  1161 |   await expect(page.getByRole("textbox", { name: "Notes", exact: true })).toHaveValue("Synthetic test: soreness after running");
  1162 |   await page.evaluate(() => { window.__deskFail = ""; });
  1163 |   await page.getByRole("button", { name: "Save pain entry" }).click();
  1164 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1165 |   await expect(page.locator(".pain-episode")).toContainText("Left knee");
  1166 |   await page.getByRole("button", { name: "Discuss episode" }).click();
  1167 |   await page.getByRole("textbox", { name: "Your question" }).fill("Should I take a rest day?");
  1168 |   await expect(page.getByRole("button", { name: "Send question" })).toBeDisabled();
  1169 |   await page.getByRole("checkbox", { name: /I agree to send/ }).check();
  1170 |   await page.getByRole("button", { name: "Send question" }).click();
  1171 |   await expect(page.locator(".chat-message.assistant")).toContainText("reducing the session");
  1172 |   await expect(page.getByRole("textbox", { name: "Your question" })).toHaveValue("");
  1173 |   await page.getByRole("button", { name: "Mark resolved" }).click();
  1174 |   await expect(page.locator(".pain-episode")).toHaveCount(0);
  1175 |   await page.getByRole("checkbox", { name: "Include resolved" }).check();
  1176 |   await page.getByRole("button", { name: "Reopen", exact: true }).click();
  1177 |   await expect(page.locator(".pain-episode")).toContainText("Active");
  1178 | });
  1179 | 
  1180 | for (const route of ["journal", "health", "health/milestones"]) {
  1181 |   test(`${route} loads backend evidence and meets accessibility checks`, async ({ page }) => {
  1182 |     const errors = [];
  1183 |     page.on("pageerror", error => errors.push(error.message));
  1184 |     await page.goto(`/desk.html#${route}`);
  1185 |     await expect(page.locator(".desk-loading")).toHaveCount(0);
  1186 |     await expect(page.locator(".error-notice")).toHaveCount(0);
  1187 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1188 |     expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  1189 |     expect(errors).toEqual([]);
  1190 |   });
  1191 | }
  1192 | 
  1193 | test("settings directory searches tasks and preserves unrelated settings", async ({ page }) => {
  1194 |   await page.goto("/desk.html#settings");
  1195 |   await page.getByRole("searchbox", { name: "Search settings" }).fill("colour");
  1196 |   await expect(page.locator(".settings-task-list a")).toHaveCount(1);
  1197 |   await page.getByRole("link", { name: /Appearance/ }).click();
  1198 |   await page.getByRole("radio", { name: "Dark", exact: true }).check();
  1199 |   await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
  1200 |   const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  1201 |   expect(settings.llmEndpoint).toBe("https://example.test/v1");
  1202 |   expect(settings.thresholds.buildWeeks).toBe(3);
  1203 |   await page.getByRole("radio", { name: "Glacier", exact: true }).check();
  1204 |   await expect(page.locator("html")).toHaveAttribute("data-desk-palette", "glacier");
  1205 | });
  1206 | 
  1207 | test("calendar settings never publish on read and retain blank credentials and sync metadata", async ({ page }) => {
  1208 |   await page.goto("/desk.html#settings/calendar");
  1209 |   await expect(page.getByRole("button", { name: "Save calendar settings" })).toBeVisible();
  1210 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "publish_calendar"))).toEqual([]);
  1211 |   await page.getByRole("checkbox", { name: "Publish calendar remotely" }).check();
  1212 |   await page.getByRole("button", { name: "Save calendar settings" }).click();
  1213 |   await expect(page.getByRole("status").last()).toContainText("Calendar settings saved");
  1214 |   const payload = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_calendar_publishing").args);
  1215 |   expect(payload.password).toBeNull();
  1216 |   expect(payload.config.lastPublished).toBe("2026-10-29T12:00:00Z");
  1217 |   await page.getByRole("checkbox", { name: "Sync this plan across devices" }).uncheck();
  1218 |   const sync = await page.evaluate(() => window.__deskCalls.find(call => call.command === "plan_sync_configure").args);
  1219 |   expect(sync).toEqual({config:{enabled:false},token:null});
  1220 |   await expect(page.getByText("synthetic-account-channel", {exact:true})).toBeVisible();
  1221 |   await expect(page.getByRole("button", { name: "Import plan", exact:true })).toBeDisabled();
  1222 | });
  1223 | 
  1224 | test("plan import only overwrites after explicit confirmation", async ({ page }) => {
  1225 |   await page.goto("/desk.html?syncConflict=1#settings/calendar");
  1226 |   await page.getByRole("button", { name: "Import plan", exact: true }).click();
  1227 |   await expect(page.getByRole("dialog", {name:"Replace the plan on this device?"})).toBeVisible();
  1228 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false]);
  1229 |   await page.getByRole("button", { name:"Keep local plan" }).click();
```