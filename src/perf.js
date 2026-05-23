const perfStore = window.__ORI_PERF__ || {
    latest: {},
    history: [],
};

window.__ORI_PERF__ = perfStore;

export function recordPerf(name, durationMs, extra = {}) {
    const sample = {
        name,
        durationMs: Number(durationMs.toFixed(2)),
        at: Date.now(),
        ...extra,
    };

    perfStore.latest[name] = sample;
    perfStore.history.push(sample);
    if (perfStore.history.length > 60) {
        perfStore.history.shift();
    }

    document.dispatchEvent(new CustomEvent('ori:perf', { detail: sample }));
    return sample;
}
