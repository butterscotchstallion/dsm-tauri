import "./App.css";
import {useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";

interface Disk {
    name: string;
    total_space: number;
    available_space: number;
}

function App() {
    const [disks, setDisks] = useState<Disk[]>([]);

    useEffect(() => {
        const loadDisks = async () => {
            try {
                const result: Disk[] = await invoke("get_disks");
                setDisks(result);
            } catch (error) {
                console.error("Failed to fetch disks:", error);
            }
        };
        loadDisks();
    }, []);

    const formatBytes = (bytes: number, decimals = 2) => {
        if (bytes === 0) return '0 Bytes';

        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

        const i = Math.floor(Math.log(bytes) / Math.log(k));

        return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
    };

  return (
      <main className="container p-8 max-w-2xl mx-auto">
          <h1 className="text-3xl font-bold text-slate-200 mb-8">Disk Space Monitor</h1>

          <div className="grid gap-6">
              {disks.map((disk, index) => {
                  const usedSpace = disk.total_space - disk.available_space;
                  const usedPercentage = disk.total_space > 0
                      ? (usedSpace / disk.total_space) * 100
                      : 0;

                  return (
                      <div key={index} className="p-6 bg-white shadow-sm rounded-xl border border-slate-200">
                          <div className="flex justify-between items-end mb-2">
                              <h2 className="font-bold text-lg text-slate-700">{disk.name || "Local Disk"}</h2>
                              <span className="text-sm font-medium text-slate-500">
                                  {usedPercentage.toFixed(1)}% Used
                              </span>
                          </div>

                          {/* Progress Bar Container */}
                          <div className="w-full bg-slate-100 rounded-full h-4 mb-4 overflow-hidden">
                              {/* Progress Bar Fill */}
                              <div
                                  className={`h-full rounded-full transition-all duration-500 ${
                                      usedPercentage > 90 ? 'bg-red-500' : usedPercentage > 75 ? 'bg-amber-500' : 'bg-blue-500'
                                  }`}
                                  style={{ width: `${usedPercentage}%` }}
                              />
                          </div>

                          <div className="flex justify-between text-sm text-slate-600 font-mono">
                              <span>Used: {formatBytes(usedSpace)}</span>
                              <span>Total: {formatBytes(disk.total_space)}</span>
                          </div>
                      </div>
                  );
              })}
          </div>
      </main>
  );
}

export default App;
