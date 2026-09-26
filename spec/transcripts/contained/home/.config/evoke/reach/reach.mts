// The spec's body under a declaration that names any host: a closed port on this machine refuses the connection,
// so what answered was the port, not a layer, and the network was open to the body.
import { connect } from "node:net"

export default () =>
  new Promise<string>((resolve, reject) => {
    const socket = connect(1, "127.0.0.1")
    socket.once("connect", () => {
      socket.destroy()
      resolve("connected to 127.0.0.1:1")
    })
    socket.once("error", (error: { code?: string }) => {
      if (error.code === "ECONNREFUSED") resolve("reached 127.0.0.1:1")
      else reject(error)
    })
  })
