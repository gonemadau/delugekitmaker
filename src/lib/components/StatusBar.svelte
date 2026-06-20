<script lang="ts">
  import { appStore } from "../stores/app.svelte";
</script>

<footer class="flex h-6 items-center justify-between border-t border-border bg-bg-1 px-3 text-[10px] text-text-muted">
  <div class="flex items-center gap-4">
    <span>v0.1.0-poc</span>
    {#if appStore.sdRoot}
      <span class="text-text-dim">SD: {appStore.sdRoot.path}</span>
    {/if}
  </div>
  <div class="flex items-center gap-4">
    {#if appStore.audioInfo}
      <span class="text-text-dim">audio: {appStore.audioInfo.sample_rate} Hz · {appStore.audioInfo.channels}ch</span>
    {:else}
      <span class="text-[color:var(--color-accent-danger)]">audio offline</span>
    {/if}
    {#if appStore.loading}
      <span class="text-accent">loading…</span>
    {/if}
    {#if appStore.activeKit}
      <span>{appStore.activeKit.drums.length} drums</span>
    {/if}
  </div>
</footer>
