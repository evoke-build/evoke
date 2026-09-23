// download saves the file a URL names under the place given or ~/Downloads, says what it saved and how big, never
// writes over a file already there, names a folder that is not there and an answer that is not a file, and leaves
// nothing behind when it is cut short.
import { equal, rejects } from "node:assert/strict"
import { access, mkdir, mkdtemp, readFile, rm } from "node:fs/promises"
import { createServer } from "node:http"
import type { Server } from "node:http"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { after, before, test } from "node:test"

import download from "../download/download.mts"

let server: Server
let base: string
let home: string
let was: string | undefined

before(async () => {
  server = createServer((request, response) => {
    if (request.url === "/files/report%20final.pdf") return response.end("twelve bytes")
    if (request.url === "/slow") {
      response.writeHead(200)
      response.write("the first chunk")
      return
    }
    response.writeHead(404)
    response.end()
  })
  await new Promise<void>(resolve => server.listen(0, "127.0.0.1", resolve))
  const address = server.address()
  if (address === null || typeof address === "string") throw new Error("no port")
  base = `http://127.0.0.1:${address.port}`
  home = await mkdtemp(join(tmpdir(), "download-"))
  was = process.env.HOME
  process.env.HOME = home
  await mkdir(join(home, "Downloads"))
})

after(async () => {
  process.env.HOME = was
  server.closeAllConnections()
  await new Promise(resolve => server.close(resolve))
  await rm(home, { recursive: true, force: true })
})

const context = (signal = new AbortController().signal) => ({ input: "", config: {}, signal })

test("a file lands in ~/Downloads under the name the URL gives it, decoded", async () => {
  equal(await download({ url: `${base}/files/report%20final.pdf` }, context()), "saved report final.pdf to ~/Downloads (12 B)")
  equal(await readFile(join(home, "Downloads/report final.pdf"), "utf8"), "twelve bytes")
})

test("a place is a path under ~, or a word for a folder there; one that is not there is named", async () => {
  await mkdir(join(home, "desktop"))
  equal(await download({ url: `${base}/files/report%20final.pdf`, to: "desktop" }, context()), "saved report final.pdf to ~/desktop (12 B)")
  await rejects(download({ url: `${base}/files/report%20final.pdf`, to: "~/nowhere" }, context()), { message: "~/nowhere does not exist" })
})

test("a hidden file is never written", async () => {
  await rejects(download({ url: `${base}/files/.env` }, context()), { message: ".env would be a hidden file" })
  await rejects(access(join(home, "Downloads/.env")))
})

test("a file already there is never written over", async () => {
  await rejects(download({ url: `${base}/files/report%20final.pdf` }, context()), { message: "report final.pdf is already in ~/Downloads" })
  equal(await readFile(join(home, "Downloads/report final.pdf"), "utf8"), "twelve bytes")
})

test("an answer that is not the file is the failure", async () => {
  await rejects(download({ url: `${base}/gone.zip` }, context()), { message: `${new URL(base).host} answered 404` })
  await rejects(access(join(home, "Downloads/gone.zip")))
})

test("a download cut short by its signal leaves nothing behind", async () => {
  const controller = new AbortController()
  setTimeout(() => controller.abort(new Error("out of time")), 100)
  await rejects(download({ url: `${base}/slow` }, context(controller.signal)), { message: "out of time" })
  await rejects(access(join(home, "Downloads/slow")))
})
