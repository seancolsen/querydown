<script lang="ts">
  import DocPage from '#lib/components/DocPage.svelte';

  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<DocPage page={data} nav={data.nav} />
