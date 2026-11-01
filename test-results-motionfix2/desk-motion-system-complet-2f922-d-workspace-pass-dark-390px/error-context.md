# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> complete motion-enabled workspace pass dark 390px
- Location: tests/desk.spec.mjs:176:7

# Error details

```
Error: health

expect(received).toEqual(expected) // deep equality

- Expected  -  1
+ Received  + 93

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
+               "bgColor": "#1b201d",
+               "contrastRatio": 1.56,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#3c403d",
+               "fontSize": "10.5pt (14px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 1.56 (foreground color: #3c403d, background color: #1b201d, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+             "relatedNodes": Array [
+               Object {
+                 "html": "<p class=\"context-note\">100% marks the configured optimized-load ceiling (1.50 times usual load). Values above this ceiling stay at 100%; the unscaled load ratio is shown above.</p>",
+                 "target": Array [
+                   ".context-note:nth-child(4)",
+                 ],
+               },
+             ],
+           },
+         ],
+         "failureSummary": "Fix any of the following:
+   Element has insufficient color contrast of 1.56 (foreground color: #3c403d, background color: #1b201d, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+         "html": "<p class=\"context-note\">100% marks the configured optimized-load ceiling (1.50 times usual load). Values above this ceiling stay at 100%; the unscaled load ratio is shown above.</p>",
+         "impact": "serious",
+         "none": Array [],
+         "target": Array [
+           ".context-note:nth-child(4)",
+         ],
+       },
+       Object {
+         "all": Array [],
+         "any": Array [
+           Object {
+             "data": Object {
+               "bgColor": "#1b201d",
+               "contrastRatio": 1.56,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#3c403d",
+               "fontSize": "10.5pt (14px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 1.56 (foreground color: #3c403d, background color: #1b201d, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+             "relatedNodes": Array [
+               Object {
+                 "html": "<p class=\"context-note\">Target weekly load: 55. </p>",
+                 "target": Array [
+                   ".context-note:nth-child(5)",
+                 ],
+               },
+             ],
+           },
+         ],
+         "failureSummary": "Fix any of the following:
+   Element has insufficient color contrast of 1.56 (foreground color: #3c403d, background color: #1b201d, font size: 10.5pt (14px), font weight: normal). Expected contrast ratio of 4.5:1",
+         "html": "<p class=\"context-note\">Target weekly load: 55. </p>",
+         "impact": "serious",
+         "none": Array [],
+         "target": Array [
+           ".context-note:nth-child(5)",
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
    - heading "Health" [active] [level=1] [ref=e41]
    - generic [ref=e42]:
      - generic [ref=e43]:
        - generic [ref=e44]:
          - text: Training and recovery
          - heading "Your training, in balance" [level=2] [ref=e45]
        - button "Log pain" [ref=e46] [cursor=pointer]
      - generic [ref=e49]:
        - navigation "Health view" [ref=e50]:
          - link "Overview" [ref=e51] [cursor=pointer]:
            - /url: "#health/overview"
          - link "Pace insights" [ref=e52] [cursor=pointer]:
            - /url: "#health/pace"
          - link "Pain journal" [ref=e53] [cursor=pointer]:
            - /url: "#health/journal"
          - link "Milestones" [ref=e54] [cursor=pointer]:
            - /url: "#health/milestones"
        - generic [ref=e55]:
          - generic [ref=e56]: Health period
          - combobox "Health period" [ref=e57] [cursor=pointer]:
            - option "Today"
            - option "7 days" [selected]
            - option "30 days"
      - generic [ref=e58]:
        - generic [ref=e59]:
          - generic [ref=e60]:
            - text: Training status
            - heading "Optimized" [level=2] [ref=e61]
            - paragraph [ref=e62]: Recent load is 1.13× your recent average — the productive zone where base fitness is increasing.
            - paragraph
          - generic [ref=e63]:
            - generic [ref=e64]:
              - term [ref=e65]: Readiness
              - definition [ref=e66]: 68%
            - generic [ref=e67]:
              - term [ref=e68]: Recovery
              - definition [ref=e69]: 61%
            - generic [ref=e70]:
              - term [ref=e71]: Injury risk
              - definition [ref=e72]: 34%
            - generic [ref=e73]:
              - term [ref=e74]: Training
              - definition [ref=e75]: 78%
        - generic [ref=e76]:
          - generic [ref=e77]:
            - generic [ref=e78]:
              - heading "Recovery and training" [level=3] [ref=e79]
              - generic [ref=e80]: Fatigued
            - figure "Readiness 68%" [ref=e81]:
              - generic [ref=e82]:
                - generic [ref=e83]: Readiness
                - strong [ref=e84]: 68%
              - img "Readiness, 7 measurements; latest 68 percent; fixed scale 0 to 100 percent" [ref=e85]:
                - generic [ref=e87]: 100%
                - generic [ref=e88]: 50%
                - generic [ref=e89]: 0%
                - 'generic "26/10/2026: 65%" [ref=e91]'
                - 'generic "27/10/2026: 68%" [ref=e92]'
                - 'generic "28/10/2026: 58%" [ref=e93]'
                - 'generic "29/10/2026: 76%" [ref=e94]'
                - 'generic "30/10/2026: 65%" [ref=e95]'
                - 'generic "31/10/2026: 69%" [ref=e96]'
                - 'generic "01/11/2026: 68%" [ref=e97]'
                - generic [ref=e98]: 26/10/2026
                - generic [ref=e99]: 01/11/2026
              - group [ref=e100]:
                - generic "Data for readiness" [ref=e101] [cursor=pointer]
            - figure "Recovery score 61%" [ref=e102]:
              - generic [ref=e103]:
                - generic [ref=e104]: Recovery score
                - strong [ref=e105]: 61%
              - img "Recovery score, 7 measurements; latest 61 percent; fixed scale 0 to 100 percent" [ref=e106]:
                - generic [ref=e108]: 100%
                - generic [ref=e109]: 50%
                - generic [ref=e110]: 0%
                - 'generic "26/10/2026: 71%" [ref=e112]'
                - 'generic "27/10/2026: 60%" [ref=e113]'
                - 'generic "28/10/2026: 49%" [ref=e114]'
                - 'generic "29/10/2026: 78%" [ref=e115]'
                - 'generic "30/10/2026: 67%" [ref=e116]'
                - 'generic "31/10/2026: 56%" [ref=e117]'
                - 'generic "01/11/2026: 61%" [ref=e118]'
                - generic [ref=e119]: 26/10/2026
                - generic [ref=e120]: 01/11/2026
              - group [ref=e121]:
                - generic "Data for recovery score" [ref=e122] [cursor=pointer]
            - figure "Training score 78%" [ref=e123]:
              - generic [ref=e124]:
                - generic [ref=e125]: Training score
                - strong [ref=e126]: 78%
              - img "Training score, 7 measurements; latest 78 percent; fixed scale 0 to 100 percent" [ref=e127]:
                - generic [ref=e129]: 100%
                - generic [ref=e130]: 50%
                - generic [ref=e131]: 0%
                - 'generic "26/10/2026: 72%" [ref=e133]'
                - 'generic "27/10/2026: 85%" [ref=e134]'
                - 'generic "28/10/2026: 68%" [ref=e135]'
                - 'generic "29/10/2026: 81%" [ref=e136]'
                - 'generic "30/10/2026: 64%" [ref=e137]'
                - 'generic "31/10/2026: 77%" [ref=e138]'
                - 'generic "01/11/2026: 78%" [ref=e139]'
                - generic [ref=e140]: 26/10/2026
                - generic [ref=e141]: 01/11/2026
              - group [ref=e142]:
                - generic "Data for training score" [ref=e143] [cursor=pointer]
            - generic [ref=e144]:
              - heading "Training load" [level=3] [ref=e145]
              - generic [ref=e146]:
                - generic [ref=e147]:
                  - term [ref=e148]: Short term
                  - definition [ref=e149]: "62"
                - generic [ref=e150]:
                  - term [ref=e151]: Long term
                  - definition [ref=e152]: "55"
                - generic [ref=e153]:
                  - term [ref=e154]: Load ratio
                  - definition [ref=e155]: "1.13"
              - figure "Training load (normalized) 75%" [ref=e156]:
                - generic [ref=e157]:
                  - generic [ref=e158]: Training load (normalized)
                  - strong [ref=e159]: 75%
                - img "Training load (normalized), 14 measurements; latest 75.33333333333333 percent; fixed scale 0 to 100 percent" [ref=e160]:
                  - generic [ref=e162]: 100%
                  - generic [ref=e163]: 50%
                  - generic [ref=e164]: 0%
                  - 'generic "05/10/2026: 49%" [ref=e166]'
                  - 'generic "06/10/2026: 53%" [ref=e167]'
                  - 'generic "07/10/2026: 55%" [ref=e168]'
                  - 'generic "08/10/2026: 54%" [ref=e169]'
                  - 'generic "09/10/2026: 59%" [ref=e170]'
                  - 'generic "10/10/2026: 63%" [ref=e171]'
                  - 'generic "11/10/2026: 61%" [ref=e172]'
                  - 'generic "12/10/2026: 67%" [ref=e173]'
                  - 'generic "13/10/2026: 70%" [ref=e174]'
                  - 'generic "14/10/2026: 69%" [ref=e175]'
                  - 'generic "15/10/2026: 73%" [ref=e176]'
                  - 'generic "16/10/2026: 77%" [ref=e177]'
                  - 'generic "17/10/2026: 74%" [ref=e178]'
                  - 'generic "18/10/2026: 75%" [ref=e179]'
                  - generic [ref=e180]: 05/10/2026
                  - generic [ref=e181]: 18/10/2026
                - group [ref=e182]:
                  - generic "Data for training load (normalized)" [ref=e183] [cursor=pointer]
              - paragraph [ref=e184]: 100% marks the configured optimized-load ceiling (1.50 times usual load). Values above this ceiling stay at 100%; the unscaled load ratio is shown above.
              - paragraph [ref=e185]: "Target weekly load: 55."
            - generic [ref=e186]:
              - heading "Fitness and volume" [level=3] [ref=e187]
              - generic [ref=e188]:
                - generic [ref=e189]:
                  - term [ref=e190]: Distance
                  - definition [ref=e191]: 52.3km
                - generic [ref=e192]:
                  - term [ref=e193]: Time
                  - definition [ref=e194]: 4h 00m
                - generic [ref=e195]:
                  - term [ref=e196]: Sessions
                  - definition [ref=e197]: "5"
                - generic [ref=e198]:
                  - term [ref=e199]: VO2 max
                  - definition [ref=e200]: "54.2"
                - generic [ref=e201]:
                  - term [ref=e202]: Threshold
                  - definition [ref=e203]: 4:38/km
              - generic [ref=e204]:
                - generic [ref=e205]:
                  - term [ref=e206]: 5 km prediction
                  - definition [ref=e207]: 20m
                - generic [ref=e208]:
                  - term [ref=e209]: 10 km prediction
                  - definition [ref=e210]: 42m
                - generic [ref=e211]:
                  - term [ref=e212]: Half marathon prediction
                  - definition [ref=e213]: 1h 33m
                - generic [ref=e214]:
                  - term [ref=e215]: Marathon prediction
                  - definition [ref=e216]: 3h 21m
            - region "Training race estimates" [ref=e218]:
              - heading "Training race estimates" [level=3] [ref=e219]
              - paragraph [ref=e220]: Plan-derived estimates / Plan baseline. Separate from watch predictions.
              - table [ref=e221]:
                - caption [ref=e222]: Race estimates from the plan threshold and accepted pace adjustment
                - rowgroup [ref=e223]:
                  - row [ref=e224]:
                    - columnheader "Distance" [ref=e225]
                    - columnheader "Estimated time" [ref=e226]
                - rowgroup [ref=e227]:
                  - row [ref=e228]:
                    - rowheader "5 km" [ref=e229]
                    - cell "22m" [ref=e230]
                  - row [ref=e231]:
                    - rowheader "10 km" [ref=e232]
                    - cell "45m" [ref=e233]
                  - row [ref=e234]:
                    - rowheader "Half marathon" [ref=e235]
                    - cell "1h 41m" [ref=e236]
                  - row [ref=e237]:
                    - rowheader "Marathon" [ref=e238]
                    - cell "3h 31m" [ref=e239]
              - group [ref=e240]:
                - generic "Estimate basis" [ref=e241] [cursor=pointer]
          - complementary [ref=e242]:
            - heading "Risk contributors" [level=3] [ref=e243]
            - generic [ref=e244]:
              - strong [ref=e245]: Load ratio (ACWR)
              - meter "Load ratio (ACWR) contribution" [ref=e246]
              - paragraph [ref=e247]: 1.13 — within optimized range
            - generic [ref=e248]:
              - strong [ref=e249]: Volume spike
              - meter "Volume spike contribution" [ref=e250]
              - paragraph [ref=e251]: within baseline (106%)
            - generic [ref=e252]:
              - strong [ref=e253]: Sleep quality
              - meter "Sleep quality contribution" [ref=e254]
              - paragraph [ref=e255]: 78/100 avg
            - generic [ref=e256]:
              - strong [ref=e257]: HRV trend
              - meter "HRV trend contribution" [ref=e258]
              - paragraph [ref=e259]: 41 ms vs 52 ms baseline
            - generic [ref=e260]:
              - strong [ref=e261]: Resting HR drift
              - meter "Resting HR drift contribution" [ref=e262]
              - paragraph [ref=e263]: 52 bpm vs 50 bpm baseline
            - generic [ref=e264]:
              - strong [ref=e265]: Recovery debt
              - meter "Recovery debt contribution" [ref=e266]
              - paragraph [ref=e267]: 71/100 recovery from sleep, HRV and resting HR
            - figure "Injury risk 34%" [ref=e268]:
              - generic [ref=e269]:
                - generic [ref=e270]: Injury risk
                - strong [ref=e271]: 34%
              - img "Injury risk, 7 measurements; latest 34 percent; fixed scale 0 to 100 percent" [ref=e272]:
                - generic [ref=e274]: 100%
                - generic [ref=e275]: 50%
                - generic [ref=e276]: 0%
                - 'generic "26/10/2026: 62%" [ref=e278]'
                - 'generic "27/10/2026: 55%" [ref=e279]'
                - 'generic "28/10/2026: 48%" [ref=e280]'
                - 'generic "29/10/2026: 41%" [ref=e281]'
                - 'generic "30/10/2026: 34%" [ref=e282]'
                - 'generic "31/10/2026: 27%" [ref=e283]'
                - 'generic "01/11/2026: 34%" [ref=e284]'
                - generic [ref=e285]: 26/10/2026
                - generic [ref=e286]: 01/11/2026
              - group [ref=e287]:
                - generic "Data for injury risk" [ref=e288] [cursor=pointer]
            - heading "Daily health" [level=3] [ref=e289]
            - generic [ref=e290]:
              - generic [ref=e291]:
                - term [ref=e292]: Resting heart rate
                - definition [ref=e293]: 54 bpm
              - generic [ref=e294]:
                - term [ref=e295]: HRV
                - definition [ref=e296]: 40 ms
              - generic [ref=e297]:
                - term [ref=e298]: Sleep
                - definition [ref=e299]: 6h 20m
              - generic [ref=e300]:
                - term [ref=e301]: Sleep score
                - definition [ref=e302]: "62"
              - generic [ref=e303]:
                - term [ref=e304]: Steps
                - definition [ref=e305]: 8,000
              - generic [ref=e306]:
                - term [ref=e307]: Stress
                - definition [ref=e308]: "28"
            - paragraph [ref=e309]: Training indicators are not a medical diagnosis. Persistent or severe pain should be assessed by a qualified clinician.
        - generic [ref=e310]:
          - heading "Daily health history" [level=3] [ref=e311]
          - generic [ref=e312]:
            - figure "Sleep duration (hours) 6.33" [ref=e313]:
              - generic [ref=e314]:
                - generic [ref=e315]: Sleep duration (hours)
                - strong [ref=e316]: "6.33"
              - img "Sleep duration (hours), 7 measurements; latest 6.333333333333333" [ref=e317]:
                - 'generic "26/10/2026: 6.53" [ref=e320]'
                - 'generic "27/10/2026: 7.75" [ref=e321]'
                - 'generic "28/10/2026: 7.47" [ref=e322]'
                - 'generic "29/10/2026: 7.18" [ref=e323]'
                - 'generic "30/10/2026: 6.90" [ref=e324]'
                - 'generic "31/10/2026: 6.62" [ref=e325]'
                - 'generic "01/11/2026: 6.33" [ref=e326]'
                - generic [ref=e327]: 26/10/2026
                - generic [ref=e328]: 01/11/2026
              - group [ref=e329]:
                - generic "Data for sleep duration (hours)" [ref=e330] [cursor=pointer]
            - figure "Heart rate variability (ms) 40.00" [ref=e331]:
              - generic [ref=e332]:
                - generic [ref=e333]: Heart rate variability (ms)
                - strong [ref=e334]: "40.00"
              - img "Heart rate variability (ms), 7 measurements; latest 40" [ref=e335]:
                - 'generic "26/10/2026: 60.00" [ref=e338]'
                - 'generic "27/10/2026: 48.00" [ref=e339]'
                - 'generic "28/10/2026: 62.00" [ref=e340]'
                - 'generic "29/10/2026: 50.00" [ref=e341]'
                - 'generic "30/10/2026: 64.00" [ref=e342]'
                - 'generic "31/10/2026: 52.00" [ref=e343]'
                - 'generic "01/11/2026: 40.00" [ref=e344]'
                - generic [ref=e345]: 26/10/2026
                - generic [ref=e346]: 01/11/2026
              - group [ref=e347]:
                - generic "Data for heart rate variability (ms)" [ref=e348] [cursor=pointer]
            - figure "Resting heart rate (bpm) 54.00" [ref=e349]:
              - generic [ref=e350]:
                - generic [ref=e351]: Resting heart rate (bpm)
                - strong [ref=e352]: "54.00"
              - img "Resting heart rate (bpm), 7 measurements; latest 54" [ref=e353]:
                - 'generic "26/10/2026: 60.00" [ref=e356]'
                - 'generic "27/10/2026: 55.00" [ref=e357]'
                - 'generic "28/10/2026: 58.00" [ref=e358]'
                - 'generic "29/10/2026: 61.00" [ref=e359]'
                - 'generic "30/10/2026: 56.00" [ref=e360]'
                - 'generic "31/10/2026: 59.00" [ref=e361]'
                - 'generic "01/11/2026: 54.00" [ref=e362]'
                - generic [ref=e363]: 26/10/2026
                - generic [ref=e364]: 01/11/2026
              - group [ref=e365]:
                - generic "Data for resting heart rate (bpm)" [ref=e366] [cursor=pointer]
          - region "Daily wearable measurements" [ref=e367]:
            - table [ref=e368]:
              - caption [ref=e369]: Daily wearable measurements
              - rowgroup [ref=e370]:
                - row [ref=e371]:
                  - columnheader "Date" [ref=e372]
                  - columnheader "Sleep" [ref=e373]
                  - columnheader "Deep sleep" [ref=e374]
                  - columnheader "REM sleep" [ref=e375]
                  - columnheader "Sleep score" [ref=e376]
                  - columnheader "HRV" [ref=e377]
                  - columnheader "Resting HR" [ref=e378]
                  - columnheader "Steps" [ref=e379]
                  - columnheader "Stress" [ref=e380]
              - rowgroup [ref=e381]:
                - row [ref=e382]:
                  - rowheader "01/11/2026" [ref=e383]
                  - cell "6h 20m" [ref=e384]
                  - cell "60 min" [ref=e385]
                  - cell "80 min" [ref=e386]
                  - cell "62" [ref=e387]
                  - cell "40" [ref=e388]
                  - cell "54" [ref=e389]
                  - cell "8000" [ref=e390]
                  - cell "28" [ref=e391]
                - row [ref=e392]:
                  - rowheader "31/10/2026" [ref=e393]
                  - cell "6h 37m" [ref=e394]
                  - cell "69 min" [ref=e395]
                  - cell "86 min" [ref=e396]
                  - cell "75" [ref=e397]
                  - cell "52" [ref=e398]
                  - cell "59" [ref=e399]
                  - cell "8971" [ref=e400]
                  - cell "39" [ref=e401]
                - row [ref=e402]:
                  - rowheader "30/10/2026" [ref=e403]
                  - cell "6h 54m" [ref=e404]
                  - cell "78 min" [ref=e405]
                  - cell "92 min" [ref=e406]
                  - cell "88" [ref=e407]
                  - cell "64" [ref=e408]
                  - cell "56" [ref=e409]
                  - cell "9942" [ref=e410]
                  - cell "28" [ref=e411]
                - row [ref=e412]:
                  - rowheader "29/10/2026" [ref=e413]
                  - cell "7h 11m" [ref=e414]
                  - cell "87 min" [ref=e415]
                  - cell "98 min" [ref=e416]
                  - cell "71" [ref=e417]
                  - cell "50" [ref=e418]
                  - cell "61" [ref=e419]
                  - cell "10913" [ref=e420]
                  - cell "39" [ref=e421]
                - row [ref=e422]:
                  - rowheader "28/10/2026" [ref=e423]
                  - cell "7h 28m" [ref=e424]
                  - cell "96 min" [ref=e425]
                  - cell "104 min" [ref=e426]
                  - cell "84" [ref=e427]
                  - cell "62" [ref=e428]
                  - cell "58" [ref=e429]
                  - cell "11884" [ref=e430]
                  - cell "28" [ref=e431]
                - row [ref=e432]:
                  - rowheader "27/10/2026" [ref=e433]
                  - cell "7h 45m" [ref=e434]
                  - cell "65 min" [ref=e435]
                  - cell "110 min" [ref=e436]
                  - cell "67" [ref=e437]
                  - cell "48" [ref=e438]
                  - cell "55" [ref=e439]
                  - cell "8855" [ref=e440]
                  - cell "39" [ref=e441]
                - row [ref=e442]:
                  - rowheader "26/10/2026" [ref=e443]
                  - cell "6h 32m" [ref=e444]
                  - cell "74 min" [ref=e445]
                  - cell "116 min" [ref=e446]
                  - cell "80" [ref=e447]
                  - cell "60" [ref=e448]
                  - cell "60" [ref=e449]
                  - cell "9826" [ref=e450]
                  - cell "28" [ref=e451]
          - group [ref=e452]:
            - generic "Training boundaries" [ref=e453] [cursor=pointer]
```

# Test source

```ts
  92  |         await session.detach();
  93  |         const removed = await page.evaluate(() => {
  94  |           let count = 0;
  95  |           for (const sheet of document.styleSheets) {
  96  |             for (let index = sheet.cssRules.length - 1; index >= 0; index--) {
  97  |               const rule = sheet.cssRules[index];
  98  |               if (rule instanceof CSSSupportsRule && rule.conditionText.includes("backdrop-filter")) { sheet.deleteRule(index); count++; }
  99  |             }
  100 |           }
  101 |           return count;
  102 |         });
  103 |         expect(removed).toBeGreaterThan(0);
  104 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  105 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  106 |         await page.keyboard.press("Escape");
  107 |         await expect(dialog).toHaveCount(0);
  108 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  109 |       });
  110 |     }
  111 |   }
  112 | 
  113 |   for (const width of [390, 1440]) {
  114 |     for (const reducedMotion of ["no-preference", "reduce"]) {
  115 |       test(`semantic reveals and compact presses ${reducedMotion} ${width}px`, async ({ page }) => {
  116 |         await page.setViewportSize({ width, height: 900 });
  117 |         await page.emulateMedia({ reducedMotion });
  118 |         await page.addInitScript(() => {
  119 |           window.__contentReveals = [];
  120 |           for (const type of ["animationstart", "animationend"]) {
  121 |             document.addEventListener(type, event => {
  122 |               if (event.animationName !== "content-arrive") return;
  123 |               const bounds = event.target.getBoundingClientRect();
  124 |               window.__contentReveals.push({ type, className: event.target.className, width: bounds.width, height: bounds.height, x: bounds.x, y: bounds.y, elapsed: event.elapsedTime, filter: getComputedStyle(event.target).filter });
  125 |             });
  126 |           }
  127 |         });
  128 |         await page.goto("/desk.html#health/overview");
  129 |         const verdict = page.locator(".health-verdict");
  130 |         await expect(verdict).toBeVisible();
  131 |         if (reducedMotion === "no-preference") {
  132 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict" && event.type === "animationend").length)).toBe(1);
  133 |           const events = await page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict"));
  134 |           expect(events[1].width).toBeCloseTo(events[0].width, 3);
  135 |           expect(events[1].height).toBeCloseTo(events[0].height, 3);
  136 |           expect(events[1].elapsed).toBe(.42);
  137 |           expect(events[0].filter).toContain("blur(");
  138 |           await expect(verdict).toHaveAttribute("data-reveal", "settled");
  139 |           await expect(verdict).toHaveCSS("filter", "none");
  140 |           await verdict.hover();
  141 |           await page.mouse.move(0, 0);
  142 |           await expect(verdict).toHaveCSS("animation-name", "none");
  143 |         } else {
  144 |           expect(await page.evaluate(() => window.__contentReveals)).toEqual([]);
  145 |           await expect(verdict).toHaveCSS("animation-name", "none");
  146 |         }
  147 |         await page.getByRole("link", { name: "Pace insights", exact: true }).click();
  148 |         const selector = page.getByRole("button", { name: "Long", exact: true });
  149 |         await expect(selector).toBeVisible();
  150 |         const bounds = await selector.boundingBox();
  151 |         await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  152 |         await page.mouse.down();
  153 |         await expect.poll(() => selector.evaluate(element => getComputedStyle(element).scale)).toBe(reducedMotion === "reduce" ? "1" : "0.96");
  154 |         expect(await selector.evaluate(element => element.offsetWidth)).toBe(Math.round(bounds.width));
  155 |         await page.mouse.up();
  156 |         await expect(selector).toHaveCSS("scale", "none");
  157 |         await expect(selector).toHaveAttribute("aria-pressed", "true");
  158 |         await page.goto("/desk.html#settings");
  159 |         await page.getByRole("link", { name: /Offline maps/ }).click();
  160 |         const editor = page.locator(".settings-editor");
  161 |         await expect(editor).toBeVisible();
  162 |         if (reducedMotion === "no-preference") {
  163 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "settings-editor" && event.type === "animationend").length)).toBe(1);
  164 |         }
  165 |         const before = await page.evaluate(() => window.__contentReveals.length);
  166 |         await page.getByLabel("Raster PMTiles archive").fill("/synthetic/new-map.pmtiles");
  167 |         expect(await page.evaluate(() => window.__contentReveals.length)).toBe(before);
  168 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  169 |         await page.screenshot({ path: `test-results/micro-motion-${reducedMotion}-${width}.png`, fullPage: true });
  170 |       });
  171 |     }
  172 |   }
  173 | 
  174 |   for (const width of [390, 1440]) {
  175 |     for (const theme of ["light", "dark"]) {
  176 |       test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
  177 |         test.setTimeout(90000);
  178 |         await page.setViewportSize({ width, height: 1000 });
  179 |         await page.emulateMedia({ reducedMotion: "no-preference" });
  180 |         const errors = [];
  181 |         page.on("pageerror", error => errors.push(error.message));
  182 |         for (const route of ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "help"]) {
  183 |           await page.goto(`/desk.html?theme=${theme}#${route}`);
  184 |           await expect(page.locator("#workspace h1")).toBeVisible();
  185 |           await expect(page.locator(".desk-loading")).toHaveCount(0);
  186 |           expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
  187 |           // axe must sample settled colours: with animations left running it
  188 |           // catches reveals and the sliding selection marker mid-flight and
  189 |           // reports blended colours that no user ever sees. finish() settles
  190 |           // CSS animations, transitions, and WAAPI springs alike.
  191 |           await page.evaluate(() => document.getAnimations().forEach(animation => { try { animation.finish(); } catch { animation.pause(); } }));
> 192 |           expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
      |                                                                                                                            ^ Error: health
  193 |           await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
  194 |         }
  195 |         await page.goto(`/desk.html?theme=${theme}#health/pace`);
  196 |         await page.getByRole("button", { name: "Log pain", exact: true }).click();
  197 |         await expect(page.getByRole("dialog")).toBeVisible();
  198 |         await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
  199 |         await page.keyboard.press("Escape");
  200 |         await expect(page.getByRole("dialog")).toHaveCount(0);
  201 |         await page.getByRole("link", { name: "Training", exact: true }).click();
  202 |         if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
  203 |         await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
  204 |         await expect(page.locator(".desk-inspector")).toBeVisible();
  205 |         await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
  206 |         await page.keyboard.press("Escape");
  207 |         await expect(page.locator(".desk-inspector")).toHaveCount(0);
  208 |         expect(errors).toEqual([]);
  209 |       });
  210 |     }
  211 |   }
  212 | 
  213 |   test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
  214 |     await page.setViewportSize({ width: 1440, height: 900 });
  215 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  216 |     await page.goto("/desk.html#training");
  217 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  218 |     for (const workspace of ["Activities", "Health", "Training", "Health"]) {
  219 |       await navigation.getByRole("link", { name: workspace, exact: true }).click();
  220 |       await expect(page.locator("#workspace h1")).toHaveText(workspace);
  221 |     }
  222 |     const active = navigation.locator("a[aria-current]");
  223 |     await expect.poll(async () => {
  224 |       const link = await active.boundingBox();
  225 |       const marker = await active.locator(".selection-marker").boundingBox();
  226 |       return Math.abs(link.x - marker.x) + Math.abs(link.width - marker.width);
  227 |     }).toBeLessThan(1);
  228 |     const help = page.getByRole("button", { name: "Help", exact: true });
  229 |     await help.hover();
  230 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  231 |     await page.keyboard.press("Escape");
  232 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  233 |     await page.mouse.move(100, 200);
  234 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  235 |     await help.focus();
  236 |     await page.keyboard.press("Tab");
  237 |     await page.keyboard.press("Shift+Tab");
  238 |     await expect(help).toBeFocused();
  239 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  240 |     await navigation.getByRole("link", { name: "Training", exact: true }).click();
  241 |     await page.locator(".week-board .session-tile").first().click();
  242 |     await expect(page.locator(".desk-inspector")).toBeVisible();
  243 |     await page.setViewportSize({ width: 390, height: 844 });
  244 |     await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  245 |     await page.emulateMedia({ reducedMotion: "reduce" });
  246 |     await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
  247 |     await page.keyboard.press("Escape");
  248 |     await expect(page.locator(".desk-inspector")).toHaveCount(0);
  249 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  250 |   });
  251 | 
  252 |   test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
  253 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  254 |     await page.addInitScript(() => {
  255 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  256 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  257 |         if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
  258 |         return invoke(command, args);
  259 |       };
  260 |     });
  261 |     await page.goto("/desk.html#settings/maps");
  262 |     const save = page.getByRole("button", { name: "Save map source", exact: true });
  263 |     await expect(save).toBeVisible();
  264 |     await page.evaluate(() => document.fonts.ready);
  265 |     const resting = await save.boundingBox();
  266 |     await save.click();
  267 |     await expect(save).toHaveAttribute("aria-busy", "true");
  268 |     await expect(save).toBeDisabled();
  269 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  270 |     expect((await save.boundingBox()).height).toBeCloseTo(resting.height, 1);
  271 |     await page.evaluate(() => window.__releaseMotionAction());
  272 |     await expect(save).toHaveAttribute("data-result", "success");
  273 |     await expect(save).toBeEnabled();
  274 |     await expect(page.getByRole("status")).toContainText("Map source saved");
  275 |     await page.evaluate(() => { window.__deskFail = "save_settings"; });
  276 |     await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
  277 |     await page.getByLabel("Raster PMTiles archive").press("Enter");
  278 |     await expect(save).toHaveAttribute("aria-busy", "true");
  279 |     await page.evaluate(() => window.__releaseMotionAction());
  280 |     await expect(save).toHaveAttribute("data-result", "error");
  281 |     await expect(page.getByRole("alert")).toContainText("Unable to save");
  282 |     await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
  283 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  284 |     await page.evaluate(() => { window.__deskFail = ""; });
  285 |     await save.press("Enter");
  286 |     await page.evaluate(() => window.__releaseMotionAction());
  287 |     await expect(save).toHaveAttribute("data-result", "success");
  288 |     const notice = page.locator(".desk-notice");
  289 |     await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
  290 |     await expect(notice).toBeAttached();
  291 |     await expect(notice).toHaveCount(0);
  292 |   });
```