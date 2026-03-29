import { describe, it, expect, beforeEach } from 'vitest';
import { useActivityStore, selectFiltered } from './activityStore';
function makeEntry(id, status = 'uploaded') {
    return {
        id,
        timestamp: Date.now(),
        status,
        path: `/files/${id}.txt`,
        filename: `${id}.txt`,
        provider: 's3',
        error: null,
    };
}
describe('activityStore', () => {
    beforeEach(() => {
        useActivityStore.getState().clear();
    });
    it('starts with empty entries', () => {
        expect(useActivityStore.getState().entries).toHaveLength(0);
    });
    it('prepends new entries (newest first)', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a'));
        addEntry(makeEntry('b'));
        const entries = useActivityStore.getState().entries;
        expect(entries[0].id).toBe('b');
        expect(entries[1].id).toBe('a');
    });
    it('caps the buffer at 200 entries', () => {
        const { addEntry } = useActivityStore.getState();
        for (let i = 0; i < 210; i++) {
            addEntry(makeEntry(String(i)));
        }
        expect(useActivityStore.getState().entries).toHaveLength(200);
        // Oldest entries were evicted; newest (209) is first
        expect(useActivityStore.getState().entries[0].id).toBe('209');
    });
    it('clears all entries', () => {
        const { addEntry, clear } = useActivityStore.getState();
        addEntry(makeEntry('a'));
        addEntry(makeEntry('b'));
        clear();
        expect(useActivityStore.getState().entries).toHaveLength(0);
    });
    it('sets and reads filter state', () => {
        useActivityStore.getState().setFilter('error');
        expect(useActivityStore.getState().filter).toBe('error');
        useActivityStore.getState().setFilter('all');
        expect(useActivityStore.getState().filter).toBe('all');
    });
});
describe('selectFiltered', () => {
    beforeEach(() => {
        useActivityStore.getState().clear();
    });
    it('returns all entries when filter is "all"', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a', 'uploaded'));
        addEntry(makeEntry('b', 'skipped'));
        addEntry(makeEntry('c', 'error'));
        useActivityStore.getState().setFilter('all');
        const result = selectFiltered(useActivityStore.getState());
        expect(result).toHaveLength(3);
    });
    it('filters to uploaded only', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a', 'uploaded'));
        addEntry(makeEntry('b', 'skipped'));
        addEntry(makeEntry('c', 'uploaded'));
        useActivityStore.getState().setFilter('uploaded');
        const result = selectFiltered(useActivityStore.getState());
        expect(result).toHaveLength(2);
        expect(result.every((e) => e.status === 'uploaded')).toBe(true);
    });
    it('filters to skipped only', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a', 'skipped'));
        addEntry(makeEntry('b', 'uploaded'));
        useActivityStore.getState().setFilter('skipped');
        const result = selectFiltered(useActivityStore.getState());
        expect(result).toHaveLength(1);
        expect(result[0].id).toBe('a');
    });
    it('error filter includes both "error" and "failed" statuses', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a', 'error'));
        addEntry(makeEntry('b', 'failed'));
        addEntry(makeEntry('c', 'uploaded'));
        useActivityStore.getState().setFilter('error');
        const result = selectFiltered(useActivityStore.getState());
        expect(result).toHaveLength(2);
        expect(result.map((e) => e.id).sort()).toEqual(['a', 'b']);
    });
    it('returns empty array when no entries match filter', () => {
        const { addEntry } = useActivityStore.getState();
        addEntry(makeEntry('a', 'uploaded'));
        useActivityStore.getState().setFilter('error');
        expect(selectFiltered(useActivityStore.getState())).toHaveLength(0);
    });
});
