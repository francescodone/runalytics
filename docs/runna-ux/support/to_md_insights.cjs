const fs = require('fs');
const d = require('./insights_raw.json');
const arr = Array.isArray(d) ? d : Object.values(d).find(Array.isArray);
let out = '';
arr.forEach((a, i) => {
  out += `\n\n===== [${String(i + 1).padStart(2, '0')}] ${a.title} =====\nURL: ${a.url}\n\n`;
  for (const p of a.parts || []) {
    if (p.t === 'img') { out += '[IMG]\n'; continue; }
    if (p.t === 'video') { out += '[VIDEO]\n'; continue; }
    if (p.t === 'table') {
      out += '[TABLE]\n' + p.rows.map(r => '  | ' + r.join(' | ') + ' |').join('\n') + '\n[/TABLE]\n';
      continue;
    }
    const prefix = { h1: '# ', h2: '## ', h3: '### ', h4: '#### ', li: '  - ', quote: '> ' }[p.t] || '';
    out += prefix + p.txt + '\n';
  }
});
fs.writeFileSync('insights_articles.md', out);
console.log('wrote insights_articles.md', out.length, 'chars,', out.split('\n').length, 'lines');
