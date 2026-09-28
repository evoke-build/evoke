// The failback stand-in of the runbook flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async () => ({ text: "failed back to the old primary" })
