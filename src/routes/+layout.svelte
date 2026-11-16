<script lang="ts">
  import "../app.css";
  import { store } from "$lib/store.svelte";

  let { children } = $props();

  // Theme resolution: auto follows the OS, light/dark are forced.
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  let systemDark = $state(mq.matches);
  mq.addEventListener("change", (e) => (systemDark = e.matches));

  $effect(() => {
    const pref = store.boot?.config.theme ?? "auto";
    const effective = pref === "auto" ? (systemDark ? "dark" : "light") : pref;
    document.documentElement.dataset.theme = effective;
  });
</script>

<div id="app-root">
  {@render children?.()}
</div>
