export default async ({ amount }: { amount: { value: number; currency: string } }) => `deposited ${amount.value} ${amount.currency}`
