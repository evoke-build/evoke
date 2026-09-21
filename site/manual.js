// Colour for the manual's command blocks, laid over what mdBook's highlighter left: a transcript gets the site's
// terminal look, a command gets its prompt. Runs after book.js, which highlights first.
(() => {
  const esc = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
  const span = (cls, s) => `<span class="${cls}">${esc(s)}</span>`
  const own = line => {
    let html = esc(line)
    html = html.replace(/^  ([a-z_][\w-]*)(?= (?:[a-z_]+=|·|\d\.\d\d))/, (m, name) => `  <span class="name">${name}</span>`)
    html = html.replace(/\b(\d\.\d\d)\b/g, (m, n) => `<span class="${parseFloat(n) >= 0.8 ? "hi" : "mid"}">${n}</span>`)
    html = html.replace(/ (&gt;) ([^&<]+)$/, (m, gt, answer) => ` ${gt} <span class="in">${answer}</span>`)
    html = html.replace(/→/g, '<span class="arrow">→</span>')
    return `<span class="own">${html}</span>`
  }
  const transcript = code => {
    const lines = code.textContent.replace(/\n$/, "").split("\n")
    code.innerHTML = lines.map(line => {
      if (line.startsWith("$ ")) return `<span class="p">$</span> ${span("in", line.slice(2))}`
      if (line.startsWith("+ ")) return span("add", line)
      if (line.startsWith("- ")) return span("del", line)
      if (line.startsWith("#")) return span("c", line)
      if (line.startsWith("  ") || /^\[\d\]$/.test(line)) return own(line)
      return span("out", line)
    }).join("\n")
    code.closest("pre").classList.add("term")
  }
  const command = code => {
    const lines = code.textContent.replace(/\n$/, "").split("\n")
    code.innerHTML = lines.map(line => {
      if (!line.trim()) return ""
      if (line.trimStart().startsWith("#")) return span("c", line)
      const [, cmd, comment] = line.match(/^(.*?)(\s+#.*)?$/)
      return `<span class="p">$</span> ${span("in", cmd)}${comment ? span("c", comment) : ""}`
    }).join("\n")
    code.closest("pre").classList.add("term")
  }
  for (const code of document.querySelectorAll("pre > code.language-text")) if (/^\$ /m.test(code.textContent)) transcript(code)
  for (const code of document.querySelectorAll("pre > code.language-bash")) command(code)
})()
