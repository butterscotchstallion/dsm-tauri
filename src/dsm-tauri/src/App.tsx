import "./App.css";
import {useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import { ArrowPathIcon, TrashIcon } from '@heroicons/react/24/solid';
interface Disk {
    name: string;
    total_space: number;
    available_space: number;
}

function App() {
    const [disks, setDisks] = useState<Disk[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

    useEffect(() => {
        loadDisks();
        // Refresh every 15 minutes
        const interval = setInterval(loadDisks, 15 * 60 * 1000);
        return () => clearInterval(interval);
    }, []);

    const loadDisks = async () => {
        setIsLoading(true);
        try {
            const result: Disk[] = await invoke("get_disks");
            setDisks(result);
            setLastUpdated(new Date());
        } catch (error) {
            console.error("Failed to fetch disks:", error);
        } finally {
            // Small delay so the user can actually see the "refreshing" state
            setTimeout(() => setIsLoading(false), 500);
        }
    };

    const formatBytes = (bytes: number, decimals = 2) => {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
    };

    const openCleanup = async () => {
        try {
            await invoke("launch_disk_cleanup");
        } catch (error) {
            console.error("Failed to launch disk cleanup:", error);
        }
    };

    return (
      <main className="container p-8 mx-auto">
          <div className="flex justify-between items-center mb-2">
              <h1 className="text-3xl font-bold text-slate-200 hover:text-slate-400">
                  Disk Space Monitor
              </h1>

              <div className="flex gap-2">
                  <button
                      title="Launch Disk Cleanup Tool"
                      onClick={openCleanup}
                      className="px-4 py-2 rounded-lg font-medium bg-slate-700 text-slate-200 hover:bg-slate-600 transition-all active:scale-95 border border-slate-600"
                  >
                      <TrashIcon className="w-4 h-4" />
                  </button>

                  <button
                      title="Refresh disk usage data"
                      onClick={loadDisks}
                      disabled={isLoading}
                      className={`rounded-lg font-medium transition-all ${
                          isLoading
                              ? 'bg-slate-700 text-slate-400 cursor-not-allowed'
                              : 'bg-blue-600 text-white hover:bg-purple-900 active:scale-95'
                      }`}
                  >
                      <ArrowPathIcon className="w-4 h-4" />
                  </button>
              </div>
          </div>

          <div className="mb-4 text-sm text-slate-400 flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${isLoading ? 'bg-amber-500 animate-pulse' : 'bg-green-500'}`}></div>
              {lastUpdated
                  ? `Last updated: ${lastUpdated.toLocaleTimeString()}`
                  : 'Initial load...'}
          </div>

          <div className="grid gap-4">
              {disks.map((disk, index) => {
                  const usedSpace = disk.total_space - disk.available_space;
                  const usedPercentage = disk.total_space > 0
                      ? (usedSpace / disk.total_space) * 100
                      : 0;
                  return (
                      <div key={index} className="p-4 bg-white shadow-sm rounded-xl border border-slate-900">
                          <div className="flex justify-between items-end mb-2">
                              <h2 className="font-bold text-lg text-slate-900">{disk.name || "Local Disk"}</h2>
                              <span className="text-sm font-medium text-slate-900">
                                  {usedPercentage.toFixed(1)}% Used
                              </span>
                          </div>

                          {/* Progress Bar Container */}
                          <div className="w-full bg-slate-200 rounded-full h-4 mb-4 overflow-hidden">
                              <div
                                  className={`h-full rounded-full transition-all duration-500 ${
                                      usedPercentage > 90 ? 'bg-red-500' : usedPercentage > 75 ? 'bg-amber-500' : 'bg-purple-900'
                                  }`}
                                  style={{ width: `${usedPercentage}%` }}
                              />
                          </div>

                          <div className="flex justify-between text-sm text-slate-900 font-mono">
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
