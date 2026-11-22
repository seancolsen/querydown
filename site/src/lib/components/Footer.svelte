<script lang="ts">
  import { githubFileUrl } from '#lib/site.ts';
</script>

<footer>
  <p>
    Querydown is open source under the
    <a href={githubFileUrl('LICENSE')}>MIT license</a>, and made by
    <a href="https://github.com/seancolsen/">Sean Colsen</a>.
  </p>
  <p>&copy; 2022&ndash;{new Date().getFullYear()} Sean Colsen.</p>
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

  p + p {
    margin-top: 0.25rem;
  }
</style>
