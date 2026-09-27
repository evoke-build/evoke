// The accounts stand-in of the new-colleague flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const ACCOUNTS = ["mail", "chat", "vpn"]

export default async ({ hire }: { hire?: { person?: string } }) => ({
  text: `${ACCOUNTS.join(", ")} created for ${hire?.person ?? "the hire"}`,
  data: { created: ACCOUNTS },
})
