import { useState } from "react";
import { ping } from "./ipc";

export default function App() {
  const [response, setResponse] = useState<string | null>(null);

  async function handlePing() {
    const result = await ping();
    setResponse(result);
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-4 bg-gray-900 text-white">
      <h1 className="text-3xl font-bold">Shadow</h1>
      <button
        onClick={handlePing}
        className="rounded bg-blue-600 px-4 py-2 hover:bg-blue-700"
      >
        Ping
      </button>
      {response !== null && (
        <p className="text-green-400">Response: {response}</p>
      )}
    </div>
  );
}
