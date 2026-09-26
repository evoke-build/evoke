export default async ({ service }: { service: string }) => ({
  text: `${service}: 1 deploy today, 4.12.0 at 13:58`,
  data: { service, deploys: [{ release: "4.12.0", at: "13:58" }] },
})
