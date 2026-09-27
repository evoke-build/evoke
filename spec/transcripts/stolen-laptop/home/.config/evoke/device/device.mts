// The device stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const DEVICE = { serial: "C02XK1ABJG5M", model: "14-inch, 2024", seen: "07:41" }

export default async ({ person }: { person: string }) => ({
  text: `${person}: serial ${DEVICE.serial}, last seen ${DEVICE.seen}`,
  data: { person, serial: DEVICE.serial, model: DEVICE.model, seen: DEVICE.seen },
})
