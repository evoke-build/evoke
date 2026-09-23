// A result whose data outgrows a pipe's buffer: 64 KiB is where a single write used to stop.
export default async () => ({
  text: "ok",
  data: { rows: Array.from({ length: 4000 }, (_, n) => ({ n, pad: "x".repeat(24) })) },
})
