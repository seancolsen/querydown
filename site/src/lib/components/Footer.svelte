<script lang="ts">
  import { GITHUB_URL } from '#lib/site.ts';
</script>

<footer>
  <p>
    Querydown is open source under the
    <a href="{GITHUB_URL}/blob/main/LICENSE">MIT license</a>, and made by
    <a href="https://github.com/seancolsen/">Sean Colsen</a>.
  </p>
</footer>

<style>
  footer {
    margin-top: 4rem;
    padding: 1.5rem 0 2.5rem;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  p {
    margin: 0;
  }
</style>
