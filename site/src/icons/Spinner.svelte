<svg class="Spinner" height="1em" viewBox="0 0 95 95">
  <path
    d="m47.49 13.56c-8.723 0-17.44 3.302-24.04 9.899-13.19 13.19-13.19 34.89 0 48.08 10.3 10.3 25.76 12.55 38.33 6.788l12.65 13.87 14.77-13.48-12.56-13.76c7.761-13.04 6.06-30.31-5.119-41.49-6.597-6.597-15.32-9.897-24.04-9.899zm0 19.87c3.562 0 7.125 1.386 9.901 4.162 5.551 5.551 5.55 14.25 0 19.8s-14.25 5.55-19.8 0-5.55-14.25 0-19.8c2.775-2.775 6.339-4.162 9.901-4.162z"
    style="fill:currentcolor;"
  />
</svg>

<style>
  .Spinner {
    display: inline-block;
    animation: spin 1.5s infinite;
    animation-timing-function: cubic-bezier(0.8, 0, 0.2, 1);
  }
  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
