<script lang="ts">
  import Button from "$lib/components/ui/button/button.svelte";
  import type { NewsTitle } from "../models/newsTitle";
    import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { invoke } from "@tauri-apps/api/core";

  const { newsTitle } : { newsTitle: NewsTitle } = $props(); 
    let loading = $state(false);
    let path = $state<string | null>(null);

    async function onGenerateVideo(_: Event) {
        loading = true;
        console.log("Generating video for:", newsTitle.title);

        try {
            path = await invoke('gen_video', { newsTitle: newsTitle });
        } catch (error) {
            console.error("Error generating video:", error);
        } finally {
            loading = false;
        }
    }

function onOpenFolder() {
    if (path) {
        revealItemInDir(path);
    } else {
        console.warn("No path to open");
    }
}

$inspect(path, "Generated video path");

</script>

<div>
    <Button onclick={onGenerateVideo} disabled={loading}>生成短视频</Button>
    {#if path}
        <Button onclick={() => openPath(path!)}>打开视频</Button>
        <Button onclick={onOpenFolder}>打开文件夹</Button>
    {/if}
</div>