// An error with a two-line message and one frame of its own, so what the loader prints is the same anywhere.
export default async () => {
  const error = new Error("first line\nsecond line of the message")
  error.stack = "Error: first line\nsecond line of the message\n    at body (throws.mts:1:1)"
  throw error
}
