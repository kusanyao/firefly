import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

// 桌面应用不需要 WebView 自带的右键菜单（刷新 / 检查 / 另存为）。
// 开发时仍可用 F12 打开开发者工具。
window.addEventListener("contextmenu", (event) => event.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
