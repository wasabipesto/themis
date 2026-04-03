import sampleData from "@data/sample_api_data.json";

const API_URL_BASE = "http://127.0.0.1:8000/";

/// Fetch data from API with fallback to local JSON
export async function getXRayDataFallback(path?: string, query?: string) {
    let data = sampleData;
    try {
        const response = await fetch(`${API_URL_BASE}/xray/sample`);
        if (response.ok) {
            data = await response.json();
        }
    } catch (error) {
        // Fall back to sample_api_data.json if dev api is down
        console.log("API unavailable, using local sample data");
        console.log(error);
    }
    return data;
}

/// Fetch data from API, no fallback
export async function getXRayData(path?: string, query?: string) {
    const response = await fetch(`${API_URL_BASE}/xray/sample`);
    return await response.json();
}
