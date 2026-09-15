import { mount } from "svelte";
import Detail from "./detail/Detail.svelte";
import "./app.css";

const detail = mount(Detail, { target: document.getElementById("detail")! });

export default detail;
