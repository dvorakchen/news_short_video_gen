<script lang="ts">
  import * as Dialog from "$lib/components/ui/dialog/";
  
  const { url, title } = $props();

</script>


  <Dialog.Root>
    <Dialog.Trigger>
      <span class="cursor-pointer hover:underline">
        {title}
      </span>
    </Dialog.Trigger>
    <Dialog.Content>
        <Dialog.Description>
            <iframe src={url} frameborder="0" title="" class=" w-screen h-screen"></iframe>
        </Dialog.Description>
    </Dialog.Content>
  </Dialog.Root>