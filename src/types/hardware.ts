export interface RuntimeProfile {
    cpuName: string;
    cpuLogicalCores: number;
    totalMemoryMb: number;
    hasAvx2: boolean;
    hasAvx512: boolean;
    hasGpu: boolean;
    gpuNames: string[];
    supportedAcceleration: string; // "CUDA", "CPU_AVX2", "CPU_BASIC"
    fallbackReason: string | null;
}
