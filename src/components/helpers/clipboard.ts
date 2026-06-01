import { EventRecord } from "../../types";

/**
 * Copy text to clipboard via the web Clipboard API
 */
export async function copyTextToClipboard(text: string): Promise<void> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
  } else {
    throw new Error("Clipboard API not available");
  }
}

/**
 * Convert a data URL (base64) to a Blob
 */
export function dataUrlToBlob(dataUrl: string): Blob {
  const [header, base64] = dataUrl.split(",");
  const mime = header.match(/:(.*?);/)?.[1] ?? "image/png";
  const bytes = atob(base64);
  const array = new Uint8Array(bytes.length);

  for (let i = 0; i < bytes.length; i++) {
    array[i] = bytes.charCodeAt(i);
  }

  return new Blob([array], { type: mime });
}

/**
 * Copy image to clipboard via the web Clipboard API
 */
export async function copyImageToClipboard(dataUrl: string): Promise<void> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.write) {
    const blob = dataUrlToBlob(dataUrl);
    const item = new ClipboardItem({ [blob.type]: blob });
    await navigator.clipboard.write([item]);
  } else {
    throw new Error("Clipboard API not available");
  }
}

/**
 * Determine the copy action based on event payload type
 */
export async function copyEventContent(event: EventRecord): Promise<void> {
  if (event.payload.type === "clipboard_image") {
    const imageData = event.payload.preview ?? event.content_hash;
    if (imageData) {
      await copyImageToClipboard(imageData);
    } else {
      throw new Error("No image data available");
    }
  } else {
    const textContent = event.payload?.content ?? JSON.stringify(event.payload);
    await copyTextToClipboard(String(textContent));
  }
}
