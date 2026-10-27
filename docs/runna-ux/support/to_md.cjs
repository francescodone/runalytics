const fs = require('fs');
const d = require('./raw_articles.json');
const arr = Array.isArray(d) ? d : Object.values(d).find(Array.isArray);
let out = '';
arr.forEach((a, i) => {
  const slug = a.url.split('/').pop();
  out += `\n\n===== [${String(i + 1).padStart(2, '0')}] ${a.title} =====\nURL: ${a.url}\n\n`;
  for (const p of a.parts || []) {
    if (p.t === 'img') { out += `[IMG alt="${p.alt}"]\n`; continue; }
    if (p.t === 'table') {
      out += '[TABLE]\n' + p.rows.map(r => '  | ' + r.join(' | ') + ' |').join('\n') + '\n[/TABLE]\n';
      continue;
    }
    const prefix = { h1: '# ', h2: '## ', h3: '### ', h4: '#### ', li: '  - ', blockquote: '> ' }[p.t] || '';
    out += prefix + p.txt + '\n';
  }
});
fs.writeFileSync('all_articles.md', out);
console.log('wrote all_articles.md', out.length, 'chars,', out.split('\n').length, 'lines');
