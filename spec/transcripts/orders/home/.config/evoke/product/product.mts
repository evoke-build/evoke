export default async ({ product }: { product: string }) => ({ text: `${product}: a book, 12.00, 40 in stock`, data: { product } })
