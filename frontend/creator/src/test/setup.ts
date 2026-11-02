import '@testing-library/jest-dom/vitest';
import {cleanup} from '@testing-library/preact';
import {afterEach, beforeEach, vi} from 'vitest';

// No backend in tests: every request is answered 401, which reads as "not
// logged in". Tests that need the API mock ../creatorApi or stub fetch again.
beforeEach(() => {
	vi.stubGlobal('fetch', vi.fn(async () => new Response(null, {status: 401})));
});

afterEach(() => {
	cleanup();
	vi.unstubAllGlobals();
});
