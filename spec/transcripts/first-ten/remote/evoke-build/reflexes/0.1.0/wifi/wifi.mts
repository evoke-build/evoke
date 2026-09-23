// A stand-in for the collection's body, which turns a Mac's Wi-Fi on or off: it says what that one says, and changes
// nothing, so the flow runs anywhere.
export default async ({ state }: { state: string }) => `wi-fi ${state}`
