// The example pages' stage: a room that answers its sentences on a loop while it is on screen, each line landing
// in the strip as the terminal printed it, and a line may change the room as it lands. Every line is what the
// binary or Node printed in a session that ran. Also the tabs beside it, and the Copy buttons.
(() => {
  for (const button of document.querySelectorAll("[data-copy]")) {
    button.addEventListener("click", async () => {
      const text = button.parentElement.querySelector("code").textContent
      try {
        await navigator.clipboard.writeText(text)
        button.textContent = "Copied"
        button.classList.add("done")
        setTimeout(() => { button.textContent = "Copy"; button.classList.remove("done") }, 1600)
      } catch {
        button.textContent = "Select it"
      }
    })
  }

  // Tabs: files, code, the app. Arrow keys move between them.
  for (const panel of document.querySelectorAll(".tabbed")) {
    const tabs = [...panel.querySelectorAll('[role="tab"]')], panes = [...panel.querySelectorAll('[role="tabpanel"]')]
    const select = i => {
      tabs.forEach((tab, j) => { tab.setAttribute("aria-selected", String(i === j)); tab.tabIndex = i === j ? 0 : -1 })
      panes.forEach((pane, j) => { pane.hidden = i !== j })
    }
    tabs.forEach((tab, i) => {
      tab.addEventListener("click", () => select(i))
      tab.addEventListener("keydown", event => {
        const move = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0
        if (!move) return
        event.preventDefault()
        const next = (i + move + tabs.length) % tabs.length
        select(next); tabs[next].focus()
      })
    })
  }

  const wait = ms => new Promise(resolve => setTimeout(resolve, ms))
  const spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
  const rich = (s, text) => {
    for (const part of text.split(/(\{(?:n|hi|mid):[^}]*\})/)) {
      const m = part.match(/^\{(n|hi|mid):([^}]*)\}$/)
      if (!m) { if (part) s.append(part); continue }
      const el = document.createElement(m[1] === "n" ? "b" : "i")
      if (m[1] === "mid") el.className = "mid"
      el.textContent = m[2]
      s.append(el)
    }
    return s
  }
  const line = (cls, text) => { const s = document.createElement("span"); s.className = "line " + cls; return text === undefined ? s : rich(s, text) }
  const prompt = () => {
    const s = line("say"), p = document.createElement("span"), input = document.createElement("span"), cursor = document.createElement("span")
    p.className = "p"; p.textContent = "$"; input.className = "in"; cursor.className = "cursor"
    s.append(p, " ", input, cursor)
    return s
  }
  const type = async (el, text) => { let typed = ""; for (const ch of text) { typed += ch; el.textContent = typed; await wait(28 + Math.random() * 40) } }
  const set = (demo, values) => { for (const [key, value] of Object.entries(values ?? {})) demo.dataset[key] = value }

  const scenes = {
    mac: { final: { volume: "40", wifi: "off", power: "asked" }, scenes: [
      { step: 0, say: 'evoke "set the volume to 40 percent"', busy: "", lines: [
        { text: '  {n:volume} level="40 percent"  {hi:0.93}', set: { volume: "40" } },
        { cls: "out", text: "volume 40%" },
      ], hold: 2800 },
      { step: 1, say: 'evoke "kill the wifi"', busy: "", lines: [
        { text: '  {n:wifi} state="off" · write · weakest: state {mid:0.72}' },
        { ask: "  Turn Wi-Fi off?  [y]es [n]o [t]each > ", answer: "t" },
        { cls: "add", text: '+ overlays/wifi.toml  [examples] "kill the wifi" = { state = "off" }', set: { wifi: "off" } },
        { cls: "out", text: "wi-fi off" },
      ], hold: 3200 },
      { step: 2, say: 'evoke "restart the computer"', busy: "", lines: [
        { text: '  {n:power} action="restart" · destructive · weakest: action {hi:0.97}', set: { power: "asked" } },
        { ask: "  Really restart now?  [y]es [n]o [t]each > ", answer: "n" },
        { text: "[2]" },
      ], hold: 3200 },
    ] },
    ops: { final: { checkout: "6", payments: "1", release: "41", bell: "quiet" }, step: 0, scenes: [
      { step: 0, say: 'evoke "scale checkout to 6 in staging"', busy: "", lines: [
        { text: '  {n:scale} service="checkout" env="staging" replicas="6"  {hi:0.90}', set: { checkout: "6" } },
        { cls: "out", text: "deployment.apps/checkout-api scaled" },
      ] },
      { step: 1, say: 'evoke "take payments down to 1 replica"', busy: "", lines: [
        { ask: "  Which environment?  [1] staging  [2] prod  [+] add one  > ", answer: "2" },
        { text: '  {n:scale} service="payments" env="prod" replicas="1"  {hi:0.95}', set: { payments: "1" } },
        { cls: "out", text: "deployment.apps/payments-svc scaled" },
      ] },
      { step: 2, say: 'evoke "roll checkout back in prod"', busy: "", lines: [
        { text: '  {n:rollback} service="checkout" env="prod" · destructive · weakest: service {hi:0.99}' },
        { ask: "  Roll checkout back in prod?  [y]es [n]o [t]each > ", answer: "y", after: { release: "41" } },
        { cls: "out", text: "deployment.apps/checkout-api rolled back" },
      ], hold: 2900 },
      { step: 0, say: 'evoke "silence db-3 for 2 hours"', busy: "", lines: [
        { text: '  {n:silence} host="db-3" duration="2 hours"  {hi:0.99}', set: { bell: "quiet" } },
        { cls: "out", text: "db-3 silenced until 01:12 AM" },
      ], hold: 3400 },
    ] },
    stream: { final: { l1: "ran", l2: "queued", l3: "none" }, scenes: [
      { say: "evoke < inbox.txt", busy: "", lines: [
        { step: 0, text: '  {n:scale} service="checkout" env="staging" replicas="6"  {hi:0.90}', set: { l1: "ran" } },
        { cls: "out", text: "deployment.apps/checkout-api scaled" },
        { step: 1, text: '  an ask needs a terminal  →  evoke "silence db-3"', set: { l2: "queued" }, pause: 600 },
        { step: 2, text: "  {n:none} {hi:1.00} · 3 more under 0.01", set: { l3: "none" } },
        { text: "[3]" },
      ], hold: 3400 },
      { say: "node queue.ts < inbox.txt", busy: "", lines: [
        { step: 0, cls: "out", text: "deployment.apps/checkout-api scaled", set: { l1: "ran" } },
        { step: 1, cls: "out", text: 'queued · ask · "silence db-3"', set: { l2: "queued" }, pause: 600 },
        { step: 2, cls: "out", text: 'queued · abstain · "what is the weather like"', set: { l3: "none" } },
      ], hold: 3400 },
    ] },
    bank: { final: { card: "checker", stamp: "yes" }, scenes: [
      { say: "node checker.ts", busy: "", lines: [
        { step: 0, cls: "out", text: 'pay payee="acme" account="ops" amount="12400" ref="invoice 8812"', set: { card: "maker" }, pause: 900 },
        { step: 1, cls: "out", text: 'pay payee="acme" account="ops" amount="12400" ref="invoice 8812" · destructive · weakest: route 1.00', set: { card: "queue" }, pause: 1300 },
        { step: 2, cls: "out", text: "paid 12400 from acc_ops, transfer tr_0f3a", set: { card: "checker", stamp: "yes" } },
      ], hold: 3400 },
      { say: 'evoke "pay Acme from ops"', busy: "", lines: [
        { step: 0, ask: "  How much?  > ", answer: "12400", after: { card: "maker" } },
        { step: 1, text: '  {n:pay} payee="acme" account="ops" amount="12400" · destructive · weakest: route {hi:1.00}', set: { card: "queue" }, pause: 700 },
        { step: 2, ask: "  Pay acme 12400 from ops?  [y]es [n]o [t]each > ", answer: "n", set: { card: "checker" }, after: { stamp: "no" } },
        { text: "[2]" },
      ], hold: 3000 },
    ] },
    runbook: { final: { drain: "ran", failover: "ran", verify: "ran" }, scenes: [
      { say: 'node runbook.ts "drain the primary"', busy: "", lines: [
        { step: 0, cls: "out", text: "primary drained", set: { drain: "ran" }, pause: 900 },
        { step: 1, ask: "Promote the replica now?  [y]es [n]o > ", answer: "y", set: { failover: "confirm" }, after: { failover: "ran" } },
        { cls: "out", text: "replica promoted", pause: 900 },
        { step: 2, cls: "out", text: "writes landing on the new primary", set: { verify: "ran" } },
      ], hold: 4200 },
    ] },
    woven: { final: { names: "1", n1: "ran", n2: "ran", wire: "on" }, step: 0, scenes: [
      { step: 0, say: 'evoke "look up dana\'s address and email them"', busy: "", before: { names: "1", n1: "off", n2: "off", wire: "off" }, lines: [
        { text: '  1  {n:contact} name="dana"  {hi:0.90}' },
        { text: "  2  {n:mail} · takes email from 1", pause: 500 },
        { cls: "out", text: "dana <dana@example.com>", set: { n1: "ran", wire: "on" }, pause: 700 },
        { text: '  2  {n:mail} to="dana@example.com"  {hi:0.88}' },
        { cls: "out", text: "drafted to dana@example.com", set: { n2: "ran" } },
      ], hold: 3400 },
      { step: 1, say: 'evoke "start a 25 minute timer and kill the lights in the den"', busy: "", before: { names: "2", n1: "off", n2: "off", wire: "off" }, lines: [
        { text: '  1  {n:timer} duration="25 minute" · write · weakest: duration {mid:0.70}' },
        { text: '  2  {n:lights} room="den" state="off"  {hi:0.85}', pause: 500 },
        { text: '  1  {n:timer} duration="25 minute" · write · weakest: duration {mid:0.70}', set: { n1: "confirm" } },
        { ask: "  Start a 25 minute timer?  [y]es [n]o [t]each > ", answer: "n", after: { n1: "declined" } },
        { text: '  2  {n:lights} room="den" state="off"  {hi:0.85} · skipped', set: { n2: "skipped" } },
        { text: "[2]" },
      ], hold: 3200 },
      { step: 2, say: 'evoke "kill the lights in the den and feed the cat"', busy: "", before: { names: "3", n1: "off", n2: "off", wire: "off" }, lines: [
        { text: '  1  {n:lights} room="den" state="off"  {hi:0.85}' },
        { text: '  2  "feed the cat" · no reflex', set: { n2: "none" } },
        { text: "[2]" },
      ], hold: 3200 },
    ] },
  }

  const demos = [...document.querySelectorAll(".demo[data-scene]")]
  const seen = new Map(demos.map(demo => [demo, { visible: false, waiters: [] }]))
  const wake = demo => { const s = seen.get(demo); if (s.visible && !document.hidden) { for (const w of s.waiters) w(); s.waiters = [] } }
  const onScreen = demo => { const s = seen.get(demo); return s.visible && !document.hidden ? Promise.resolve() : new Promise(resolve => s.waiters.push(resolve)) }
  const watcher = new IntersectionObserver(entries => { for (const entry of entries) { seen.get(entry.target).visible = entry.isIntersecting; wake(entry.target) } }, { threshold: .25 })
  document.addEventListener("visibilitychange", () => { for (const demo of demos) wake(demo) })

  const play = async (demo, { scenes: acts }) => {
    const strip = demo.querySelector(".strip"), trail = demo.querySelectorAll(".trail b")
    const initial = { ...demo.dataset }
    const step = i => { if (i !== undefined) trail.forEach((b, j) => b.classList.toggle("on", j === i)) }
    const show = s => { strip.append(s); strip.scrollTop = strip.scrollHeight; requestAnimationFrame(() => s.classList.add("show")); return s }
    for (;;) {
      for (const key of Object.keys(demo.dataset)) delete demo.dataset[key]
      set(demo, initial)
      for (const scene of acts) {
        await onScreen(demo)
        step(scene.step); set(demo, scene.before)
        strip.classList.add("clear"); await wait(240); strip.replaceChildren(); strip.classList.remove("clear")
        await wait(420)
        const typed = show(prompt())
        await type(typed.querySelector(".in"), scene.say)
        await wait(220); typed.querySelector(".cursor").remove()
        if (scene.busy !== undefined) {
          const busy = show(line("own"))
          for (let i = 0; i < 10; i++) { busy.textContent = "  " + spinner[i] + (scene.busy ? " " + scene.busy : ""); await wait(65) }
          busy.remove()
        }
        for (const entry of scene.lines) {
          step(entry.step)
          if (entry.ask !== undefined) {
            const s = show(line("own", entry.ask)); set(demo, entry.set)
            const input = document.createElement("span"); input.className = "in"; s.append(input)
            await wait(520); await type(input, entry.answer); await wait(260); set(demo, entry.after)
          } else {
            show(line(entry.cls ?? "own", entry.text)); set(demo, entry.set)
          }
          await wait(entry.pause ?? 170)
        }
        set(demo, scene.set)
        await wait(scene.hold ?? 2600)
      }
    }
  }

  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    for (const demo of demos) { const { final, step } = scenes[demo.dataset.scene], marks = demo.querySelectorAll(".trail b"); set(demo, final); marks[step ?? marks.length - 1].classList.add("on") }
  } else {
    for (const demo of demos) { watcher.observe(demo); play(demo, scenes[demo.dataset.scene]) }
  }
})()
