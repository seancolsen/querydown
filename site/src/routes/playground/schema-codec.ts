/**
 * Encodes a customized schema for the playground's URL: deflated, then written in URL-safe base64.
 * (The sample schema is never encoded. A URL without a schema means the sample schema.)
 */

async function transform(
  bytes: Uint8Array<ArrayBuffer>,
  stream: TransformStream,
): Promise<Uint8Array> {
  const transformed = new Blob([bytes]).stream().pipeThrough(stream);
  return new Uint8Array(await new Response(transformed).arrayBuffer());
}

export async function encodeSchema(schema: string): Promise<string> {
  const bytes = await transform(
    new TextEncoder().encode(schema),
    new CompressionStream('deflate-raw'),
  );
  const binary = Array.from(bytes, (byte) => String.fromCharCode(byte)).join('');
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

export async function decodeSchema(encoded: string): Promise<string> {
  const binary = atob(encoded.replaceAll('-', '+').replaceAll('_', '/'));
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  const inflated = await transform(bytes, new DecompressionStream('deflate-raw'));
  return new TextDecoder().decode(inflated);
}
