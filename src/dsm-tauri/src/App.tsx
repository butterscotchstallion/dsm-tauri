import {useEffect, useState} from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./tailwind.css";
import "./App.css";

function App() {

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    //setGreetMsg(await invoke("greet", { name }));
  }

  useEffect(async () => {
      await invoke("getDisks");
  }, []);

  return (
    <main className="container">
        <h1 className="text-3xl font-bold text-slate-300">Disk Space Monitor</h1>

        hello world
    </main>
  );
}

export default App;
