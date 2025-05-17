import fs from "fs";
import path from "path";
const CACHE_DIR = path.resolve(process.cwd(), "cache");

// Type to represent cache data with loading state
type CacheEntry<T> = {
  data: T | null;
  lastUpdated: number;
};

// In-memory cache storage
const memoryCache = new Map<string, CacheEntry<any>>();

// Track in-flight promises for data being loaded
const pendingPromises = new Map<string, Promise<any>>();

// Cache options interface
interface CacheOptions {
  expirationMs?: number;
  saveToMemory?: boolean;
  saveToDisk?: boolean;
}

// Default cache options
const defaultCacheOptions: CacheOptions = {
  expirationMs: 7 * 24 * 60 * 60 * 1000,
  saveToMemory: true,
  saveToDisk: true,
};

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

// Get cache entry, initializing if necessary
function getMemCacheEntry<T>(cacheKey: string): CacheEntry<T> {
  if (!memoryCache.has(cacheKey)) {
    memoryCache.set(cacheKey, {
      data: null,
      lastUpdated: 0,
    });
  }
  return memoryCache.get(cacheKey) as CacheEntry<T>;
}

// Save data to disk cache in chunks, flattening maps into arrays
export function saveToDiskCache<T>(cacheKey: string, data: T): void {
  console.log(`Saving ${cacheKey} to disk cache`);
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
      timestamp: Date.now(),
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

    console.log(
      `Saved disk cache (${arrayData.length} items, ${numChunks} chunks) for ${cacheKey}`,
    );
  } catch (error) {
    console.warn(`Failed to save disk cache for ${cacheKey}: ${error}`);
  }
}

// Load data from disk cache, handling chunked data
export async function loadFromDiskCache<T>(cacheKey: string): Promise<{
  data: T | null;
  timestamp: number;
}> {
  try {
    // Check if we have a metadata file for chunked data
    const metaFile = path.join(CACHE_DIR, `${cacheKey}_meta.json`);

    // This is chunked data, read the metadata
    const metadata = JSON.parse(fs.readFileSync(metaFile, "utf8"));
    console.log(`Loading chunked disk cache for ${cacheKey}`);

    // Load all chunks in parallel
    const chunkPromises = Array.from(
      { length: metadata.numChunks },
      async (_, i) => {
        const chunkFile = path.join(CACHE_DIR, `${cacheKey}_chunk_${i}.json`);
        return fs.promises
          .readFile(chunkFile, "utf8")
          .then((data) => JSON.parse(data))
          .catch((err) => {
            console.warn(
              `Missing chunk ${i} for ${cacheKey}, cache incomplete: ${err}`,
            );
            throw new Error(`Missing chunk ${i}`);
          });
      },
    );

    // Wait for all chunks to load
    const chunks = await Promise.all(chunkPromises);

    // Combine all chunks
    const resultArray = chunks.flat();

    // Convert back to Map if the original was a Map
    let result: T;
    if (metadata.isMap) {
      console.log(
        `Loaded disk cache (${resultArray.length} entries, map) for ${cacheKey}`,
      );
      result = new Map(resultArray) as T;
    } else {
      console.log(
        `Loaded disk cache (${resultArray.length} items, array) for ${cacheKey}`,
      );
      result = resultArray as T;
    }

    return {
      data: result,
      timestamp: metadata.timestamp || 0,
    };
  } catch (error) {
    // Data files likely do not exist, return empty
    return { data: null, timestamp: 0 };
  }
}

// Save data to cache (both memory and disk)
export function saveToCache<T>(
  cacheKey: string,
  data: T,
  options: CacheOptions = {},
): void {
  const opts = { ...defaultCacheOptions, ...options };

  // Update memory cache
  if (opts.saveToMemory) {
    const cacheEntry = getMemCacheEntry<T>(cacheKey);
    cacheEntry.data = data;
    cacheEntry.lastUpdated = Date.now();
  }

  // Save to disk if requested
  if (opts.saveToDisk) {
    saveToDiskCache(cacheKey, data);
  }
}

// Load data from cache (checking memory first, then disk)
export async function loadFromCache<T>(
  cacheKey: string,
  options: CacheOptions = {},
): Promise<T | null> {
  const opts = { ...defaultCacheOptions, ...options };
  const now = Date.now();

  // Check memory cache first
  if (opts.saveToMemory) {
    const cacheEntry = getMemCacheEntry<T>(cacheKey);

    // If data exists in memory and isn't expired
    if (
      cacheEntry.data !== null &&
      (opts.expirationMs === undefined ||
        now - cacheEntry.lastUpdated < opts.expirationMs)
    ) {
      return cacheEntry.data;
    }
  }

  // If not in memory or expired, try disk cache
  if (opts.saveToDisk) {
    const { data, timestamp } = await loadFromDiskCache<T>(cacheKey);

    // If data exists on disk and isn't expired
    if (
      data !== null &&
      (opts.expirationMs === undefined || now - timestamp < opts.expirationMs)
    ) {
      // Update memory cache if disk load succeeded
      if (opts.saveToMemory) {
        const cacheEntry = getMemCacheEntry<T>(cacheKey);
        cacheEntry.data = data;
        cacheEntry.lastUpdated = timestamp;
      }

      return data;
    }
  }

  return null;
}

// Try to get cached data or load it using the provided function
export async function getOrFetchData<T>(
  cacheKey: string,
  fetchFn: () => Promise<T>,
  options: CacheOptions = {},
): Promise<T> {
  const opts = { ...defaultCacheOptions, ...options };

  // Try to get from cache first
  const cachedData = await loadFromCache<T>(cacheKey, opts);
  if (cachedData !== null) {
    return cachedData;
  }

  // Check if a promise for this data is already pending
  // If so, return it and this request will track that promise
  if (pendingPromises.has(cacheKey)) {
    console.log(`Reusing pending promise for ${cacheKey}`);
    return pendingPromises.get(cacheKey) as Promise<T>;
  }

  // Create a new promise for this data fetch
  const fetchPromise = (async () => {
    try {
      // Fetch fresh data
      console.log(`Fetching fresh data for ${cacheKey}`);
      const data = await fetchFn();

      // Save to cache
      saveToCache(cacheKey, data, opts);

      return data;
    } catch (error) {
      console.error(`Error fetching data for ${cacheKey}:`, error);
      throw error;
    } finally {
      // Remove this promise from pending promises
      pendingPromises.delete(cacheKey);
    }
  })();

  // Store the promise so other calls can reuse it
  pendingPromises.set(cacheKey, fetchPromise);

  return fetchPromise;
}
