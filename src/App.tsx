import { Button } from "@/components/ui/button";
import ScanButton from "@/components/ScanButton";

function App() {

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-10 bg-white text-black">
      <div className="text-center mb-8">
        <h1 className="text-4xl font-bold mb-4">
          Welcome to AI OS 👋
        </h1>
        <p className="text-xl text-gray-600 max-w-md">
          Scan and analyze your files with AI-powered insights
        </p>
      </div>

      <ScanButton />
    </div>
  );
}

export default App;
