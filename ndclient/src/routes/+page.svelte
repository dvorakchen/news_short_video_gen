<script lang="ts">
  import Button from '$lib/components/ui/button/button.svelte';
  import { Switch } from "$lib/components/ui/switch/";
  import { Label } from "$lib/components/ui/label/";
  import * as Table from "$lib/components/ui/table/";
  import { invoke } from '@tauri-apps/api/core';
  import type { NewsTitle } from '@/lib/models/newsTitle';
  import NewsWindow from '@/lib/components/news-window.svelte';
  import GenVideo from '@/lib/components/gen-video.svelte';

  import '@/index.css';

  let hotNewsTitles = $state<NewsTitle[]>([]);
  let autoRefresh = $state(false);

  async function getHotNews() {
    console.log("Fetching hot news...");
    hotNewsTitles = await invoke('get_hot_news');
  }

  async function onRefresh(e: Event) {
    hotNewsTitles = [];
    await getHotNews();
  }

  $effect(() => {
    getHotNews();
    if (autoRefresh) {
      const interval = setInterval(async () => {
        await getHotNews();
      }, 1000);

      return () => clearInterval(interval);
    }
  });
</script>

<main class="flex flex-col p-8">
  <section>
    <div class="flex gap-4">
      <Button onclick={onRefresh}>刷新澎湃新闻热点</Button>
      
      <div class="flex items-center space-x-2">
        <Switch id="autoRefresh" bind:checked={autoRefresh} />
        <Label for="autoRefresh">自动刷新(30分钟)</Label>
      </div>
    </div>
  </section>
  
  <section>
    <Table.Root>
     <Table.Caption>澎湃新闻热点</Table.Caption>
     <Table.Header>
      <Table.Row>
       <Table.Head class="max-w-2xs">标题</Table.Head>
       <Table.Head class="text-right"></Table.Head>
      </Table.Row>
     </Table.Header>
     <Table.Body>
      {#each hotNewsTitles as title}
      <Table.Row>
       <Table.Cell class="font-medium">
        <NewsWindow url={title.url} title={title.title} />
       </Table.Cell>
       <Table.Cell class="text-right flex space-x-2">
        <GenVideo newsTitle={title} />
       </Table.Cell>
      </Table.Row>
      {/each}
     </Table.Body>
    </Table.Root>
  </section>

</main>