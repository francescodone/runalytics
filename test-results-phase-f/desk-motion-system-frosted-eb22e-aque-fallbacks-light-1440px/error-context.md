# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> frosted surfaces and opaque fallbacks light 1440px
- Location: tests/desk.spec.mjs:40:7

# Error details

```
Error: expect(received).toEqual(expected) // deep equality

- Expected  -  1
+ Received  + 58

- Array []
+ Array [
+   Object {
+     "description": "Ensure the contrast between foreground and background colors meets WCAG 2 AA minimum contrast ratio thresholds",
+     "help": "Elements must meet minimum color contrast ratio thresholds",
+     "helpUrl": "https://dequeuniversity.com/rules/axe/4.14/color-contrast?application=playwright",
+     "id": "color-contrast",
+     "impact": "serious",
+     "nodes": Array [
+       Object {
+         "all": Array [],
+         "any": Array [
+           Object {
+             "data": Object {
+               "bgColor": "#fdfdfd",
+               "contrastRatio": 3.24,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#898f8c",
+               "fontSize": "10.5pt (14px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 3.24 (foreground color: #898f8c, background color: #fdfdfd, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+             "relatedNodes": Array [
+               Object {
+                 "html": "<input required=\"\" max=\"2026-11-01\" type=\"date\" value=\"2026-11-01\">",
+                 "target": Array [
+                   "input[max=\"2026-11-01\"]",
+                 ],
+               },
+             ],
+           },
+         ],
+         "failureSummary": "Fix any of the following:
+   Element has insufficient color contrast of 3.24 (foreground color: #898f8c, background color: #fdfdfd, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+         "html": "<input required=\"\" max=\"2026-11-01\" type=\"date\" value=\"2026-11-01\">",
+         "impact": "serious",
+         "none": Array [],
+         "target": Array [
+           "input[max=\"2026-11-01\"]",
+         ],
+       },
+     ],
+     "tags": Array [
+       "cat.color",
+       "wcag2aa",
+       "wcag143",
+       "TTv5",
+       "TT13.c",
+       "EN-301-549",
+       "EN-9.1.4.3",
+       "ACT",
+       "RGAAv4",
+       "RGAA-3.2.1",
+     ],
+   },
+ ]
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - link "Skip to workspace" [ref=e3] [cursor=pointer]:
    - /url: "#workspace"
  - banner [ref=e4]:
    - link "Runalytics training" [ref=e5] [cursor=pointer]:
      - /url: "#training"
      - generic [ref=e6]: Runalytics
    - navigation "Workspaces" [ref=e7]:
      - link "Training" [ref=e8] [cursor=pointer]:
        - /url: "#training"
      - link "Activities" [ref=e12] [cursor=pointer]:
        - /url: "#journal"
      - link "Health" [ref=e19] [cursor=pointer]:
        - /url: "#health"
    - generic [ref=e23]:
      - button "Sync activities" [ref=e24] [cursor=pointer]
      - button "Help" [ref=e29] [cursor=pointer]
      - button "Settings" [ref=e34] [cursor=pointer]
  - main [ref=e39]:
    - generic [ref=e40]:
      - heading "Health" [level=1] [ref=e41]
      - generic [ref=e42]: COROS connected
    - generic [ref=e43]:
      - generic [ref=e44]:
        - generic [ref=e45]:
          - text: Training and recovery
          - heading "Your training, in balance" [level=2] [ref=e46]
        - button "Log pain" [ref=e47] [cursor=pointer]
      - generic [ref=e50]:
        - navigation "Health view" [ref=e51]:
          - link "Overview" [ref=e52] [cursor=pointer]:
            - /url: "#health/overview"
          - link "Pace insights" [ref=e53] [cursor=pointer]:
            - /url: "#health/pace"
          - link "Pain journal" [ref=e54] [cursor=pointer]:
            - /url: "#health/journal"
          - link "Milestones" [ref=e55] [cursor=pointer]:
            - /url: "#health/milestones"
        - generic [ref=e56]:
          - generic [ref=e57]: Health period
          - combobox "Health period" [ref=e58] [cursor=pointer]:
            - option "Today"
            - option "7 days" [selected]
            - option "30 days"
      - generic [ref=e59]:
        - generic [ref=e60]:
          - generic [ref=e61]:
            - text: Training status
            - heading "Optimized" [level=2] [ref=e62]
            - paragraph [ref=e63]: Recent load is 1.13× your recent average — the productive zone where base fitness is increasing.
            - paragraph
          - generic [ref=e64]:
            - generic [ref=e65]:
              - term [ref=e66]: Readiness
              - definition [ref=e67]: 68%
            - generic [ref=e68]:
              - term [ref=e69]: Recovery
              - definition [ref=e70]: 61%
            - generic [ref=e71]:
              - term [ref=e72]: Injury risk
              - definition [ref=e73]: 34%
            - generic [ref=e74]:
              - term [ref=e75]: Training
              - definition [ref=e76]: 78%
        - generic [ref=e77]:
          - generic [ref=e78]:
            - generic [ref=e79]:
              - heading "Recovery and training" [level=3] [ref=e80]
              - generic [ref=e81]: Fatigued
            - figure "Readiness 68%" [ref=e82]:
              - generic [ref=e83]:
                - generic [ref=e84]: Readiness
                - strong [ref=e85]: 68%
              - img "Readiness, 7 measurements; latest 68 percent; fixed scale 0 to 100 percent" [ref=e86]:
                - generic [ref=e88]: 100%
                - generic [ref=e89]: 50%
                - generic [ref=e90]: 0%
                - 'generic "26/10/2026: 65%" [ref=e92]'
                - 'generic "27/10/2026: 68%" [ref=e93]'
                - 'generic "28/10/2026: 58%" [ref=e94]'
                - 'generic "29/10/2026: 76%" [ref=e95]'
                - 'generic "30/10/2026: 65%" [ref=e96]'
                - 'generic "31/10/2026: 69%" [ref=e97]'
                - 'generic "01/11/2026: 68%" [ref=e98]'
                - generic [ref=e99]: 26/10/2026
                - generic [ref=e100]: 01/11/2026
              - group [ref=e101]:
                - generic "Data for readiness" [ref=e102] [cursor=pointer]
            - figure "Recovery score 61%" [ref=e103]:
              - generic [ref=e104]:
                - generic [ref=e105]: Recovery score
                - strong [ref=e106]: 61%
              - img "Recovery score, 7 measurements; latest 61 percent; fixed scale 0 to 100 percent" [ref=e107]:
                - generic [ref=e109]: 100%
                - generic [ref=e110]: 50%
                - generic [ref=e111]: 0%
                - 'generic "26/10/2026: 71%" [ref=e113]'
                - 'generic "27/10/2026: 60%" [ref=e114]'
                - 'generic "28/10/2026: 49%" [ref=e115]'
                - 'generic "29/10/2026: 78%" [ref=e116]'
                - 'generic "30/10/2026: 67%" [ref=e117]'
                - 'generic "31/10/2026: 56%" [ref=e118]'
                - 'generic "01/11/2026: 61%" [ref=e119]'
                - generic [ref=e120]: 26/10/2026
                - generic [ref=e121]: 01/11/2026
              - group [ref=e122]:
                - generic "Data for recovery score" [ref=e123] [cursor=pointer]
            - figure "Training score 78%" [ref=e124]:
              - generic [ref=e125]:
                - generic [ref=e126]: Training score
                - strong [ref=e127]: 78%
              - img "Training score, 7 measurements; latest 78 percent; fixed scale 0 to 100 percent" [ref=e128]:
                - generic [ref=e130]: 100%
                - generic [ref=e131]: 50%
                - generic [ref=e132]: 0%
                - 'generic "26/10/2026: 72%" [ref=e134]'
                - 'generic "27/10/2026: 85%" [ref=e135]'
                - 'generic "28/10/2026: 68%" [ref=e136]'
                - 'generic "29/10/2026: 81%" [ref=e137]'
                - 'generic "30/10/2026: 64%" [ref=e138]'
                - 'generic "31/10/2026: 77%" [ref=e139]'
                - 'generic "01/11/2026: 78%" [ref=e140]'
                - generic [ref=e141]: 26/10/2026
                - generic [ref=e142]: 01/11/2026
              - group [ref=e143]:
                - generic "Data for training score" [ref=e144] [cursor=pointer]
            - generic [ref=e145]:
              - heading "Training load" [level=3] [ref=e146]
              - generic [ref=e147]:
                - generic [ref=e148]:
                  - term [ref=e149]: Short term
                  - definition [ref=e150]: "62"
                - generic [ref=e151]:
                  - term [ref=e152]: Long term
                  - definition [ref=e153]: "55"
                - generic [ref=e154]:
                  - term [ref=e155]: Load ratio
                  - definition [ref=e156]: "1.13"
              - figure "Training load (normalized) 75%" [ref=e157]:
                - generic [ref=e158]:
                  - generic [ref=e159]: Training load (normalized)
                  - strong [ref=e160]: 75%
                - img "Training load (normalized), 14 measurements; latest 75.33333333333333 percent; fixed scale 0 to 100 percent" [ref=e161]:
                  - generic [ref=e163]: 100%
                  - generic [ref=e164]: 50%
                  - generic [ref=e165]: 0%
                  - 'generic "05/10/2026: 49%" [ref=e167]'
                  - 'generic "06/10/2026: 53%" [ref=e168]'
                  - 'generic "07/10/2026: 55%" [ref=e169]'
                  - 'generic "08/10/2026: 54%" [ref=e170]'
                  - 'generic "09/10/2026: 59%" [ref=e171]'
                  - 'generic "10/10/2026: 63%" [ref=e172]'
                  - 'generic "11/10/2026: 61%" [ref=e173]'
                  - 'generic "12/10/2026: 67%" [ref=e174]'
                  - 'generic "13/10/2026: 70%" [ref=e175]'
                  - 'generic "14/10/2026: 69%" [ref=e176]'
                  - 'generic "15/10/2026: 73%" [ref=e177]'
                  - 'generic "16/10/2026: 77%" [ref=e178]'
                  - 'generic "17/10/2026: 74%" [ref=e179]'
                  - 'generic "18/10/2026: 75%" [ref=e180]'
                  - generic [ref=e181]: 05/10/2026
                  - generic [ref=e182]: 18/10/2026
                - group [ref=e183]:
                  - generic "Data for training load (normalized)" [ref=e184] [cursor=pointer]
              - paragraph [ref=e185]: 100% marks the configured optimized-load ceiling (1.50 times usual load). Values above this ceiling stay at 100%; the unscaled load ratio is shown above.
              - paragraph [ref=e186]: "Target weekly load: 55."
            - generic [ref=e187]:
              - heading "Fitness and volume" [level=3] [ref=e188]
              - generic [ref=e189]:
                - generic [ref=e190]:
                  - term [ref=e191]: Distance
                  - definition [ref=e192]: 52.3km
                - generic [ref=e193]:
                  - term [ref=e194]: Time
                  - definition [ref=e195]: 4h 00m
                - generic [ref=e196]:
                  - term [ref=e197]: Sessions
                  - definition [ref=e198]: "5"
                - generic [ref=e199]:
                  - term [ref=e200]: VO2 max
                  - definition [ref=e201]: "54.2"
                - generic [ref=e202]:
                  - term [ref=e203]: Threshold
                  - definition [ref=e204]: 4:38/km
              - generic [ref=e205]:
                - generic [ref=e206]:
                  - term [ref=e207]: 5 km prediction
                  - definition [ref=e208]: 20m
                - generic [ref=e209]:
                  - term [ref=e210]: 10 km prediction
                  - definition [ref=e211]: 42m
                - generic [ref=e212]:
                  - term [ref=e213]: Half marathon prediction
                  - definition [ref=e214]: 1h 33m
                - generic [ref=e215]:
                  - term [ref=e216]: Marathon prediction
                  - definition [ref=e217]: 3h 21m
            - region "Training race estimates" [ref=e219]:
              - heading "Training race estimates" [level=3] [ref=e220]
              - paragraph [ref=e221]: Plan-derived estimates / Plan baseline. Separate from watch predictions.
              - table [ref=e222]:
                - caption [ref=e223]: Race estimates from the plan threshold and accepted pace adjustment
                - rowgroup [ref=e224]:
                  - row [ref=e225]:
                    - columnheader "Distance" [ref=e226]
                    - columnheader "Estimated time" [ref=e227]
                - rowgroup [ref=e228]:
                  - row [ref=e229]:
                    - rowheader "5 km" [ref=e230]
                    - cell "22m" [ref=e231]
                  - row [ref=e232]:
                    - rowheader "10 km" [ref=e233]
                    - cell "45m" [ref=e234]
                  - row [ref=e235]:
                    - rowheader "Half marathon" [ref=e236]
                    - cell "1h 41m" [ref=e237]
                  - row [ref=e238]:
                    - rowheader "Marathon" [ref=e239]
                    - cell "3h 31m" [ref=e240]
              - group [ref=e241]:
                - generic "Estimate basis" [ref=e242] [cursor=pointer]
          - complementary [ref=e243]:
            - heading "Risk contributors" [level=3] [ref=e244]
            - generic [ref=e245]:
              - strong [ref=e246]: Load ratio (ACWR)
              - meter "Load ratio (ACWR) contribution" [ref=e247]
              - paragraph [ref=e248]: 1.13 — within optimized range
            - generic [ref=e249]:
              - strong [ref=e250]: Volume spike
              - meter "Volume spike contribution" [ref=e251]
              - paragraph [ref=e252]: within baseline (106%)
            - generic [ref=e253]:
              - strong [ref=e254]: Sleep quality
              - meter "Sleep quality contribution" [ref=e255]
              - paragraph [ref=e256]: 78/100 avg
            - generic [ref=e257]:
              - strong [ref=e258]: HRV trend
              - meter "HRV trend contribution" [ref=e259]
              - paragraph [ref=e260]: 41 ms vs 52 ms baseline
            - generic [ref=e261]:
              - strong [ref=e262]: Resting HR drift
              - meter "Resting HR drift contribution" [ref=e263]
              - paragraph [ref=e264]: 52 bpm vs 50 bpm baseline
            - generic [ref=e265]:
              - strong [ref=e266]: Recovery debt
              - meter "Recovery debt contribution" [ref=e267]
              - paragraph [ref=e268]: 71/100 recovery from sleep, HRV and resting HR
            - figure "Injury risk 34%" [ref=e269]:
              - generic [ref=e270]:
                - generic [ref=e271]: Injury risk
                - strong [ref=e272]: 34%
              - img "Injury risk, 7 measurements; latest 34 percent; fixed scale 0 to 100 percent" [ref=e273]:
                - generic [ref=e275]: 100%
                - generic [ref=e276]: 50%
                - generic [ref=e277]: 0%
                - 'generic "26/10/2026: 62%" [ref=e279]'
                - 'generic "27/10/2026: 55%" [ref=e280]'
                - 'generic "28/10/2026: 48%" [ref=e281]'
                - 'generic "29/10/2026: 41%" [ref=e282]'
                - 'generic "30/10/2026: 34%" [ref=e283]'
                - 'generic "31/10/2026: 27%" [ref=e284]'
                - 'generic "01/11/2026: 34%" [ref=e285]'
                - generic [ref=e286]: 26/10/2026
                - generic [ref=e287]: 01/11/2026
              - group [ref=e288]:
                - generic "Data for injury risk" [ref=e289] [cursor=pointer]
            - heading "Daily health" [level=3] [ref=e290]
            - generic [ref=e291]:
              - generic [ref=e292]:
                - term [ref=e293]: Resting heart rate
                - definition [ref=e294]: 54 bpm
              - generic [ref=e295]:
                - term [ref=e296]: HRV
                - definition [ref=e297]: 40 ms
              - generic [ref=e298]:
                - term [ref=e299]: Sleep
                - definition [ref=e300]: 6h 20m
              - generic [ref=e301]:
                - term [ref=e302]: Sleep score
                - definition [ref=e303]: "62"
              - generic [ref=e304]:
                - term [ref=e305]: Steps
                - definition [ref=e306]: 8,000
              - generic [ref=e307]:
                - term [ref=e308]: Stress
                - definition [ref=e309]: "28"
            - paragraph [ref=e310]: Training indicators are not a medical diagnosis. Persistent or severe pain should be assessed by a qualified clinician.
        - generic [ref=e311]:
          - heading "Daily health history" [level=3] [ref=e312]
          - generic [ref=e313]:
            - figure "Sleep duration (hours) 6.33" [ref=e314]:
              - generic [ref=e315]:
                - generic [ref=e316]: Sleep duration (hours)
                - strong [ref=e317]: "6.33"
              - img "Sleep duration (hours), 7 measurements; latest 6.333333333333333" [ref=e318]:
                - 'generic "26/10/2026: 6.53" [ref=e321]'
                - 'generic "27/10/2026: 7.75" [ref=e322]'
                - 'generic "28/10/2026: 7.47" [ref=e323]'
                - 'generic "29/10/2026: 7.18" [ref=e324]'
                - 'generic "30/10/2026: 6.90" [ref=e325]'
                - 'generic "31/10/2026: 6.62" [ref=e326]'
                - 'generic "01/11/2026: 6.33" [ref=e327]'
                - generic [ref=e328]: 26/10/2026
                - generic [ref=e329]: 01/11/2026
              - group [ref=e330]:
                - generic "Data for sleep duration (hours)" [ref=e331] [cursor=pointer]
            - figure "Heart rate variability (ms) 40.00" [ref=e332]:
              - generic [ref=e333]:
                - generic [ref=e334]: Heart rate variability (ms)
                - strong [ref=e335]: "40.00"
              - img "Heart rate variability (ms), 7 measurements; latest 40" [ref=e336]:
                - 'generic "26/10/2026: 60.00" [ref=e339]'
                - 'generic "27/10/2026: 48.00" [ref=e340]'
                - 'generic "28/10/2026: 62.00" [ref=e341]'
                - 'generic "29/10/2026: 50.00" [ref=e342]'
                - 'generic "30/10/2026: 64.00" [ref=e343]'
                - 'generic "31/10/2026: 52.00" [ref=e344]'
                - 'generic "01/11/2026: 40.00" [ref=e345]'
                - generic [ref=e346]: 26/10/2026
                - generic [ref=e347]: 01/11/2026
              - group [ref=e348]:
                - generic "Data for heart rate variability (ms)" [ref=e349] [cursor=pointer]
            - figure "Resting heart rate (bpm) 54.00" [ref=e350]:
              - generic [ref=e351]:
                - generic [ref=e352]: Resting heart rate (bpm)
                - strong [ref=e353]: "54.00"
              - img "Resting heart rate (bpm), 7 measurements; latest 54" [ref=e354]:
                - 'generic "26/10/2026: 60.00" [ref=e357]'
                - 'generic "27/10/2026: 55.00" [ref=e358]'
                - 'generic "28/10/2026: 58.00" [ref=e359]'
                - 'generic "29/10/2026: 61.00" [ref=e360]'
                - 'generic "30/10/2026: 56.00" [ref=e361]'
                - 'generic "31/10/2026: 59.00" [ref=e362]'
                - 'generic "01/11/2026: 54.00" [ref=e363]'
                - generic [ref=e364]: 26/10/2026
                - generic [ref=e365]: 01/11/2026
              - group [ref=e366]:
                - generic "Data for resting heart rate (bpm)" [ref=e367] [cursor=pointer]
          - region "Daily wearable measurements" [ref=e368]:
            - table [ref=e369]:
              - caption [ref=e370]: Daily wearable measurements
              - rowgroup [ref=e371]:
                - row [ref=e372]:
                  - columnheader "Date" [ref=e373]
                  - columnheader "Sleep" [ref=e374]
                  - columnheader "Deep sleep" [ref=e375]
                  - columnheader "REM sleep" [ref=e376]
                  - columnheader "Sleep score" [ref=e377]
                  - columnheader "HRV" [ref=e378]
                  - columnheader "Resting HR" [ref=e379]
                  - columnheader "Steps" [ref=e380]
                  - columnheader "Stress" [ref=e381]
              - rowgroup [ref=e382]:
                - row [ref=e383]:
                  - rowheader "01/11/2026" [ref=e384]
                  - cell "6h 20m" [ref=e385]
                  - cell "60 min" [ref=e386]
                  - cell "80 min" [ref=e387]
                  - cell "62" [ref=e388]
                  - cell "40" [ref=e389]
                  - cell "54" [ref=e390]
                  - cell "8000" [ref=e391]
                  - cell "28" [ref=e392]
                - row [ref=e393]:
                  - rowheader "31/10/2026" [ref=e394]
                  - cell "6h 37m" [ref=e395]
                  - cell "69 min" [ref=e396]
                  - cell "86 min" [ref=e397]
                  - cell "75" [ref=e398]
                  - cell "52" [ref=e399]
                  - cell "59" [ref=e400]
                  - cell "8971" [ref=e401]
                  - cell "39" [ref=e402]
                - row [ref=e403]:
                  - rowheader "30/10/2026" [ref=e404]
                  - cell "6h 54m" [ref=e405]
                  - cell "78 min" [ref=e406]
                  - cell "92 min" [ref=e407]
                  - cell "88" [ref=e408]
                  - cell "64" [ref=e409]
                  - cell "56" [ref=e410]
                  - cell "9942" [ref=e411]
                  - cell "28" [ref=e412]
                - row [ref=e413]:
                  - rowheader "29/10/2026" [ref=e414]
                  - cell "7h 11m" [ref=e415]
                  - cell "87 min" [ref=e416]
                  - cell "98 min" [ref=e417]
                  - cell "71" [ref=e418]
                  - cell "50" [ref=e419]
                  - cell "61" [ref=e420]
                  - cell "10913" [ref=e421]
                  - cell "39" [ref=e422]
                - row [ref=e423]:
                  - rowheader "28/10/2026" [ref=e424]
                  - cell "7h 28m" [ref=e425]
                  - cell "96 min" [ref=e426]
                  - cell "104 min" [ref=e427]
                  - cell "84" [ref=e428]
                  - cell "62" [ref=e429]
                  - cell "58" [ref=e430]
                  - cell "11884" [ref=e431]
                  - cell "28" [ref=e432]
                - row [ref=e433]:
                  - rowheader "27/10/2026" [ref=e434]
                  - cell "7h 45m" [ref=e435]
                  - cell "65 min" [ref=e436]
                  - cell "110 min" [ref=e437]
                  - cell "67" [ref=e438]
                  - cell "48" [ref=e439]
                  - cell "55" [ref=e440]
                  - cell "8855" [ref=e441]
                  - cell "39" [ref=e442]
                - row [ref=e443]:
                  - rowheader "26/10/2026" [ref=e444]
                  - cell "6h 32m" [ref=e445]
                  - cell "74 min" [ref=e446]
                  - cell "116 min" [ref=e447]
                  - cell "80" [ref=e448]
                  - cell "60" [ref=e449]
                  - cell "60" [ref=e450]
                  - cell "9826" [ref=e451]
                  - cell "28" [ref=e452]
          - group [ref=e453]:
            - generic "Training boundaries" [ref=e454] [cursor=pointer]
      - dialog "Log pain" [ref=e455]:
        - generic [ref=e456]:
          - generic [ref=e457]:
            - heading "Log pain" [level=2] [ref=e458]
            - button "Close Log pain" [active] [ref=e459] [cursor=pointer]
          - generic [ref=e464]:
            - generic [ref=e465]:
              - generic [ref=e466]:
                - text: Body region
                - combobox "Body region" [ref=e467] [cursor=pointer]:
                  - option "Select a region" [selected]
                  - option "Neck"
                  - option "Left shoulder"
                  - option "Right shoulder"
                  - option "Chest"
                  - option "Left pec"
                  - option "Right pec"
                  - option "Left biceps"
                  - option "Right biceps"
                  - option "Left triceps"
                  - option "Right triceps"
                  - option "Left forearm / wrist"
                  - option "Right forearm / wrist"
                  - option "Core / abs"
                  - option "Left obliques"
                  - option "Right obliques"
                  - option "Upper back / traps"
                  - option "Left lat"
                  - option "Right lat"
                  - option "Lower back"
                  - option "Left lower back"
                  - option "Right lower back"
                  - option "Left glute / hip"
                  - option "Right glute / hip"
                  - option "Left hip flexor"
                  - option "Right hip flexor"
                  - option "Left quad"
                  - option "Right quad"
                  - option "Left IT band"
                  - option "Right IT band"
                  - option "Left hamstring"
                  - option "Right hamstring"
                  - option "Left knee"
                  - option "Right knee"
                  - option "Left shin"
                  - option "Right shin"
                  - option "Left calf"
                  - option "Right calf"
                  - option "Left Achilles"
                  - option "Right Achilles"
                  - option "Left ankle / foot"
                  - option "Right ankle / foot"
                  - option "Other"
              - generic [ref=e468]:
                - text: "Intensity: 3/10"
                - 'slider "Intensity: 3/10" [ref=e469]': "3"
              - generic [ref=e470]:
                - text: Started on
                - textbox "Started on" [ref=e471]: 2026-11-01
              - generic [ref=e472]:
                - text: Notes
                - textbox "Notes" [ref=e473]
            - generic [ref=e474]:
              - button "Cancel" [ref=e475] [cursor=pointer]
              - button "Save pain entry" [ref=e477] [cursor=pointer]
```

# Test source

```ts
  1   | import { test, expect } from "@playwright/test";
  2   | import AxeBuilder from "@axe-core/playwright";
  3   | import { installDeskFixture } from "./desk.fixture.mjs";
  4   | 
  5   | test.beforeEach(async ({ page }) => installDeskFixture(page));
  6   | 
  7   | test.describe("motion system", () => {
  8   |   test("primary action depth is bounded, interruptible and preference-aware", async ({ page }) => {
  9   |     await page.setViewportSize({ width: 1440, height: 1000 });
  10  |     await page.emulateMedia({ reducedMotion: "no-preference" });
  11  |     await page.goto("/desk.html#health");
  12  |     const action = page.getByRole("button", { name: "Log pain", exact: true });
  13  |     const content = action.locator(".button-content");
  14  |     const rest = await action.boundingBox();
  15  |     await action.hover({ position: { x: rest.width - 4, y: 4 } });
  16  |     await expect.poll(() => content.evaluate(element => Number(element.style.transform.match(/rotateY\(([-.\d]+)deg\)/)?.[1] ?? 0))).toBeGreaterThan(.5);
  17  |     const angles = await content.evaluate(element => [...element.style.transform.matchAll(/rotate[XY]\(([-.\d]+)deg\)/g)].map(match => Math.abs(Number(match[1]))));
  18  |     expect(angles).toHaveLength(2);
  19  |     expect(angles.every(angle => angle <= 1.5)).toBe(true);
  20  |     expect(await action.boundingBox()).toEqual(rest);
  21  |     await action.dispatchEvent("pointercancel", { pointerType: "mouse" });
  22  |     await expect.poll(() => content.evaluate(element => element.style.transform)).not.toMatch(/rotate[XY]|translateY/);
  23  |     await action.hover({ position: { x: 4, y: 4 } });
  24  |     await expect.poll(() => content.evaluate(element => element.style.transform)).toContain("rotateY");
  25  |     await page.emulateMedia({ reducedMotion: "reduce" });
  26  |     await expect(content).toHaveCSS("transform", "none");
  27  |     await page.emulateMedia({ reducedMotion: "no-preference" });
  28  |     await action.dispatchEvent("pointermove", { pointerType: "touch", clientX: rest.x + 4, clientY: rest.y + 4 });
  29  |     await expect.poll(() => content.evaluate(element => element.style.transform)).not.toMatch(/rotate[XY]|translateY/);
  30  |     await page.mouse.move(20, 200);
  31  |     await action.focus();
  32  |     await page.keyboard.press("Enter");
  33  |     await expect(page.getByRole("dialog", { name: "Log pain" })).toBeVisible();
  34  |     await page.keyboard.press("Escape");
  35  |     await expect(action).toBeFocused();
  36  |   });
  37  | 
  38  |   for (const width of [390, 1440]) {
  39  |     for (const theme of ["light", "dark"]) {
  40  |       test(`frosted surfaces and opaque fallbacks ${theme} ${width}px`, async ({ page }) => {
  41  |         await page.setViewportSize({ width, height: 900 });
  42  |         await page.emulateMedia({ reducedMotion: "no-preference", colorScheme: theme });
  43  |         await page.goto(`/desk.html?theme=${theme}#health`);
  44  |         await expect(page.locator(".health-verdict")).toBeVisible();
  45  |         for (const selector of [".desk-toolbar", ".health-verdict", ".health-analysis"]) {
  46  |           await expect(page.locator(selector)).toHaveCSS("backdrop-filter", /blur\(/);
  47  |         }
  48  |         // The header's glass sits on a ::before layer, not on the header
  49  |         // itself: backdrop-filter on the header would make it the containing
  50  |         // block for the fixed mobile nav island and pin it to the top bar.
  51  |         const appbarGlass = await page.evaluate(() => {
  52  |           const style = getComputedStyle(document.querySelector(".desk-appbar"), "::before");
  53  |           const [, , , a] = style.backgroundColor.match(/[\d.]+/g) ?? [];
  54  |           return { filter: style.backdropFilter, alpha: a === undefined ? 1 : Number(a) };
  55  |         });
  56  |         expect(appbarGlass.filter).toBe("blur(20px) saturate(1.35)");
  57  |         expect(appbarGlass.alpha).toBeLessThan(1);
  58  |         if (width === 390) {
  59  |           await expect(page.locator(".desk-navigation")).toHaveCSS("backdrop-filter", "blur(20px) saturate(1.35)");
  60  |           const navigation = await page.locator(".desk-navigation").boundingBox();
  61  |           // The island floats: inset from the screen edges and above the
  62  |           // bottom, as a rounded pill with content scrolling underneath.
  63  |           expect(Math.round(navigation.y + navigation.height)).toBeLessThan(900);
  64  |           expect(navigation.x).toBeGreaterThan(0);
  65  |           await expect(page.locator(".desk-navigation")).toHaveCSS("border-radius", "999px");
  66  |           const settings = page.getByRole("button", { name: "Settings", exact: true });
  67  |           await settings.click();
  68  |           await expect(page.locator("#workspace h1")).toHaveText("Settings");
  69  |           await page.getByRole("link", { name: "Health", exact: true }).click();
  70  |         }
  71  |         await page.getByRole("button", { name: "Log pain", exact: true }).click();
  72  |         const dialog = page.getByRole("dialog", { name: "Log pain" });
  73  |         await expect(dialog).toHaveCSS("backdrop-filter", "blur(20px) saturate(1.35)");
> 74  |         expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
      |                                                                                                                   ^ Error: expect(received).toEqual(expected) // deep equality
  75  |         await page.screenshot({ path: `test-results/glass-${theme}-${width}.png`, animations: "disabled" });
  76  |         await page.emulateMedia({ contrast: "more" });
  77  |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  78  |         for (const selector of [".desk-toolbar", ".health-verdict", ".health-analysis"]) await expect(page.locator(selector)).toHaveCSS("backdrop-filter", "none");
  79  |         const surface = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--desk-surface").trim());
  80  |         const opaqueColor = await page.evaluate(value => { const probe = document.createElement("span"); probe.style.color = value; document.body.append(probe); const color = getComputedStyle(probe).color; probe.remove(); return color; }, surface);
  81  |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  82  |         await page.emulateMedia({ contrast: "no-preference" });
  83  |         const session = await page.context().newCDPSession(page);
  84  |         await session.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
  85  |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  86  |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  87  |         await session.send("Emulation.setEmulatedMedia", { features: [] });
  88  |         await session.detach();
  89  |         const removed = await page.evaluate(() => {
  90  |           let count = 0;
  91  |           for (const sheet of document.styleSheets) {
  92  |             for (let index = sheet.cssRules.length - 1; index >= 0; index--) {
  93  |               const rule = sheet.cssRules[index];
  94  |               if (rule instanceof CSSSupportsRule && rule.conditionText.includes("backdrop-filter")) { sheet.deleteRule(index); count++; }
  95  |             }
  96  |           }
  97  |           return count;
  98  |         });
  99  |         expect(removed).toBeGreaterThan(0);
  100 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  101 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  102 |         await page.keyboard.press("Escape");
  103 |         await expect(dialog).toHaveCount(0);
  104 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  105 |       });
  106 |     }
  107 |   }
  108 | 
  109 |   for (const width of [390, 1440]) {
  110 |     for (const reducedMotion of ["no-preference", "reduce"]) {
  111 |       test(`semantic reveals and compact presses ${reducedMotion} ${width}px`, async ({ page }) => {
  112 |         await page.setViewportSize({ width, height: 900 });
  113 |         await page.emulateMedia({ reducedMotion });
  114 |         await page.addInitScript(() => {
  115 |           window.__contentReveals = [];
  116 |           for (const type of ["animationstart", "animationend"]) {
  117 |             document.addEventListener(type, event => {
  118 |               if (event.animationName !== "content-arrive") return;
  119 |               const bounds = event.target.getBoundingClientRect();
  120 |               window.__contentReveals.push({ type, className: event.target.className, width: bounds.width, height: bounds.height, x: bounds.x, y: bounds.y, elapsed: event.elapsedTime, filter: getComputedStyle(event.target).filter });
  121 |             });
  122 |           }
  123 |         });
  124 |         await page.goto("/desk.html#health/overview");
  125 |         const verdict = page.locator(".health-verdict");
  126 |         await expect(verdict).toBeVisible();
  127 |         if (reducedMotion === "no-preference") {
  128 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict" && event.type === "animationend").length)).toBe(1);
  129 |           const events = await page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict"));
  130 |           expect(events[1].width).toBeCloseTo(events[0].width, 3);
  131 |           expect(events[1].height).toBeCloseTo(events[0].height, 3);
  132 |           expect(events[1].elapsed).toBe(.42);
  133 |           expect(events[0].filter).toContain("blur(");
  134 |           await expect(verdict).toHaveAttribute("data-reveal", "settled");
  135 |           await expect(verdict).toHaveCSS("filter", "none");
  136 |           await verdict.hover();
  137 |           await page.mouse.move(0, 0);
  138 |           await expect(verdict).toHaveCSS("animation-name", "none");
  139 |         } else {
  140 |           expect(await page.evaluate(() => window.__contentReveals)).toEqual([]);
  141 |           await expect(verdict).toHaveCSS("animation-name", "none");
  142 |         }
  143 |         await page.getByRole("link", { name: "Pace insights", exact: true }).click();
  144 |         const selector = page.getByRole("button", { name: "Long", exact: true });
  145 |         await expect(selector).toBeVisible();
  146 |         const bounds = await selector.boundingBox();
  147 |         await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  148 |         await page.mouse.down();
  149 |         await expect.poll(() => selector.evaluate(element => getComputedStyle(element).scale)).toBe(reducedMotion === "reduce" ? "1" : "0.96");
  150 |         expect(await selector.evaluate(element => element.offsetWidth)).toBe(Math.round(bounds.width));
  151 |         await page.mouse.up();
  152 |         await expect(selector).toHaveCSS("scale", "none");
  153 |         await expect(selector).toHaveAttribute("aria-pressed", "true");
  154 |         await page.goto("/desk.html#settings");
  155 |         await page.getByRole("link", { name: /Offline maps/ }).click();
  156 |         const editor = page.locator(".settings-editor");
  157 |         await expect(editor).toBeVisible();
  158 |         if (reducedMotion === "no-preference") {
  159 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "settings-editor" && event.type === "animationend").length)).toBe(1);
  160 |         }
  161 |         const before = await page.evaluate(() => window.__contentReveals.length);
  162 |         await page.getByLabel("Raster PMTiles archive").fill("/synthetic/new-map.pmtiles");
  163 |         expect(await page.evaluate(() => window.__contentReveals.length)).toBe(before);
  164 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  165 |         await page.screenshot({ path: `test-results/micro-motion-${reducedMotion}-${width}.png`, fullPage: true });
  166 |       });
  167 |     }
  168 |   }
  169 | 
  170 |   for (const width of [390, 1440]) {
  171 |     for (const theme of ["light", "dark"]) {
  172 |       test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
  173 |         test.setTimeout(90000);
  174 |         await page.setViewportSize({ width, height: 1000 });
```