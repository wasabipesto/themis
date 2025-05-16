import fs from "fs";
import path from "path";
const CACHE_DIR = path.resolve(process.cwd(), "cache");

// Ensure cache directory exists
function ensureCacheDir() {
  if (!fs.existsSync(CACHE_DIR)) {
    try {
      fs.mkdirSync(CACHE_DIR, { recursive: true });
    } catch (error) {
      throw new Error(`Failed to create cache directory: ${error}`);
    }
  }
}

// Save data to disk cache in chunks, flattening maps into arrays
export function saveToCache<T>(cacheKey: string, data: T): void {
  try {
    ensureCacheDir();
    const chunkSize = 100_000;

    // Convert Map to array of entries if needed
    let isMap = false;
    if (data instanceof Map) {
      data = Array.from(data.entries()) as any;
      isMap = true;
    }
    if (!Array.isArray(data)) {
      throw new Error("Unsupported data type");
    }

    // Save metadata file
    const metaFile = path.join(CACHE_DIR, `${cacheKey}_meta.json`);
    const arrayData = data as any[];
    const numChunks = Math.ceil(arrayData.length / chunkSize);

    // Save metadata
    const metadata = {
      type: "chunked",
      isMap,
      totalLength: arrayData.length,
      numChunks,
      chunkSize,
    };
    fs.writeFileSync(metaFile, JSON.stringify(metadata), "utf8");

    // Save each chunk
    for (let i = 0; i < numChunks; i++) {
      const chunkStart = i * chunkSize;
      const chunkEnd = Math.min((i + 1) * chunkSize, arrayData.length);
      const chunk = arrayData.slice(chunkStart, chunkEnd);

      const chunkFile = path.join(CACHE_DIR, `${cacheKey}_chunk_${i}.json`);
      fs.writeFileSync(chunkFile, JSON.stringify(chunk), "utf8");
    }

    console.log(`Saved cache (${numChunks} chunks) for ${cacheKey}`);
  } catch (error) {
    console.warn(`Failed to save cache for ${cacheKey}: ${error}`);
  }
}

// Load data from disk cache, handling chunked data
export function loadFromCache<T>(cacheKey: string): T | null {
  try {
    // Check if we have a metadata file for chunked data
    const metaFile = path.join(CACHE_DIR, `${cacheKey}_meta.json`);

    // This is chunked data, read the metadata
    const metadata = JSON.parse(fs.readFileSync(metaFile, "utf8"));
    console.log(`Loading chunked cache for ${cacheKey}`);

    // Reconstruct array from chunks
    let resultArray: any[] = [];

    for (let i = 0; i < metadata.numChunks; i++) {
      const chunkFile = path.join(CACHE_DIR, `${cacheKey}_chunk_${i}.json`);
      if (!fs.existsSync(chunkFile)) {
        console.warn(`Missing chunk ${i} for ${cacheKey}, cache incomplete`);
        return null;
      }

      const chunkData = JSON.parse(fs.readFileSync(chunkFile, "utf8"));
      resultArray = resultArray.concat(chunkData);
    }

    // Convert back to Map if the original was a Map
    if (metadata.isMap) {
      console.log(
        `Loaded cache (${resultArray.length} entries, map) for ${cacheKey}`,
      );
      return new Map(resultArray) as T;
    }

    console.log(
      `Loaded cache (${resultArray.length} items, array) for ${cacheKey}`,
    );
    return resultArray as T;
  } catch (error) {
    console.warn(`Failed to load cache for ${cacheKey}: ${error}`);
    return null;
  }
}
