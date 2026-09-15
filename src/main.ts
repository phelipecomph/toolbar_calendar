import { mount } from "svelte";
import Strip from "./strip/Strip.svelte";
import "./app.css";

const app = mount(Strip, { target: document.getElementById("app")! });

export default app;
