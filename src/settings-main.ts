import { mount } from "svelte";
import Settings from "./settings/Settings.svelte";
import "./app.css";

const settings = mount(Settings, { target: document.getElementById("settings")! });

export default settings;
