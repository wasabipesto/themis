import fs from "fs";
import path from "path";
const CACHE_DIR = path.resolve(process.cwd(), "cache");

// Type to represent cache data with loading state
type CacheEntry<T> = {
  data: T | null;
  loading: boolean;
  lastUpdated: number;
};

// In-memory cache storage
const memoryCache = new Map<string, CacheEntry<any>>();

// Cache options interface
interface CacheOptions {
  expirationMs?: number;
  skipMemory?: boolean;
  skipDisk?: boolean;
}

// Default cache options
const defaultCacheOptions: CacheOptions = {
  expirationMs: 7 * 24 * 60 * 60 * 1000,
  skipMemory: false,
  skipDisk: false,
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
function getCacheEntry<T>(cacheKey: string): CacheEntry<T> {
  if (!memoryCache.has(cacheKey)) {
    memoryCache.set(cacheKey, {
      data: null,
      loading: false,
      lastUpdated: 0,
    });
  }
  return memoryCache.get(cacheKey) as CacheEntry<T>;
}

// Save data to disk cache in chunks, flattening maps into arrays
export function saveToDiskCache<T>(cacheKey: string, data: T): void {
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

    console.log(`Saved disk cache (${numChunks} chunks) for ${cacheKey}`);
  } catch (error) {
    console.warn(`Failed to save disk cache for ${cacheKey}: ${error}`);
  }
}

// Load data from disk cache, handling chunked data
export function loadFromDiskCache<T>(cacheKey: string): {
  data: T | null;
  timestamp: number;
} {
  try {
    // Check if we have a metadata file for chunked data
    const metaFile = path.join(CACHE_DIR, `${cacheKey}_meta.json`);

    // This is chunked data, read the metadata
    const metadata = JSON.parse(fs.readFileSync(metaFile, "utf8"));
    console.log(`Loading chunked disk cache for ${cacheKey}`);

    // Reconstruct array from chunks
    let resultArray: any[] = [];

    for (let i = 0; i < metadata.numChunks; i++) {
      const chunkFile = path.join(CACHE_DIR, `${cacheKey}_chunk_${i}.json`);
      if (!fs.existsSync(chunkFile)) {
        console.warn(`Missing chunk ${i} for ${cacheKey}, cache incomplete`);
        return { data: null, timestamp: 0 };
      }

      const chunkData = JSON.parse(fs.readFileSync(chunkFile, "utf8"));
      resultArray = resultArray.concat(chunkData);
    }

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
    console.warn(`Failed to load disk cache for ${cacheKey}: ${error}`);
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
  if (!opts.skipMemory) {
    const cacheEntry = getCacheEntry<T>(cacheKey);
    cacheEntry.data = data;
    cacheEntry.loading = false;
    cacheEntry.lastUpdated = Date.now();
  }

  // Save to disk if requested
  if (!opts.skipDisk) {
    saveToDiskCache(cacheKey, data);
  }
}

// Load data from cache (checking memory first, then disk)
export function loadFromCache<T>(
  cacheKey: string,
  options: CacheOptions = {},
): T | null {
  const opts = { ...defaultCacheOptions, ...options };
  const now = Date.now();

  // Check memory cache first
  if (!opts.skipMemory) {
    const cacheEntry = getCacheEntry<T>(cacheKey);

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
  if (!opts.skipDisk) {
    const { data, timestamp } = loadFromDiskCache<T>(cacheKey);

    // If data exists on disk and isn't expired
    if (
      data !== null &&
      (opts.expirationMs === undefined || now - timestamp < opts.expirationMs)
    ) {
      // Update memory cache if disk load succeeded
      if (!opts.skipMemory) {
        const cacheEntry = getCacheEntry<T>(cacheKey);
        cacheEntry.data = data;
        cacheEntry.lastUpdated = timestamp;
      }

      return data;
    }
  }

  return null;
}

// Check if a cache key is currently being loaded
export function isLoading(cacheKey: string): boolean {
  return memoryCache.has(cacheKey)
    ? (memoryCache.get(cacheKey) as CacheEntry<any>).loading
    : false;
}

// Set loading state for a cache key
export function setLoading(cacheKey: string, loading: boolean): void {
  const cacheEntry = getCacheEntry<any>(cacheKey);
  cacheEntry.loading = loading;
}

// Try to get cached data or load it using the provided function
export async function getOrFetchData<T>(
  cacheKey: string,
  fetchFn: () => Promise<T>,
  options: CacheOptions = {},
): Promise<T> {
  const opts = { ...defaultCacheOptions, ...options };

  // Try to get from cache first
  const cachedData = loadFromCache<T>(cacheKey, opts);
  if (cachedData !== null) {
    return cachedData;
  }

  // Check if already loading
  if (isLoading(cacheKey)) {
    // Wait until the data is available
    return new Promise((resolve, reject) => {
      const checkInterval = setInterval(() => {
        if (!isLoading(cacheKey)) {
          clearInterval(checkInterval);
          const data = loadFromCache<T>(cacheKey, opts);
          if (data !== null) {
            resolve(data);
          } else {
            reject(new Error(`Cache loading failed for ${cacheKey}`));
          }
        }
      }, 500);
    });
  }

  // Set loading state
  setLoading(cacheKey, true);

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
    // Clear loading state
    setLoading(cacheKey, false);
  }
}
