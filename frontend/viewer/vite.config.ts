import { coverageConfigDefaults, defineConfig } from 'vitest/config';
import preactPreset from '@preact/preset-vite';
import tailwindcss from '@tailwindcss/vite';

interface DevServerEnv {
	bindAddr: string | undefined;
	port: number | undefined;
	publicPort: number | undefined;
	publicHost: string;
}

function intOrUndefined(value: string | undefined): number | undefined {
	if (value === undefined || value === '') return undefined;
	const parsed = parseInt(value, 10);
	if (Number.isNaN(parsed)) return undefined;
	return parsed;
}

function getEnv(name: string): string | undefined {
	const p = globalThis as unknown as {
		process?: { env?: Record<string, string | undefined> };
	};
	return p.process?.env?.[name];
}

function parseViewerEnv(): DevServerEnv {
	return {
		bindAddr: getEnv('CR_BIND_ADDR'),
		port: intOrUndefined(getEnv('CR_VIEWER_PORT')),
		publicPort: intOrUndefined(getEnv('CR_PORT')),
		publicHost: getEnv('CR_VIEWER_HOST') ?? 'app.crlocal',
	};
}

const env = parseViewerEnv();
const isProxyMode = env.publicPort !== undefined;

let hmrConfig: { protocol: 'wss'; host: string; clientPort: number } | undefined;
if (isProxyMode) {
	const publicPort = env.publicPort;
	if (publicPort === undefined) {
		throw new Error('CR_PORT is required when proxy mode is active');
	}
	hmrConfig = {
		protocol: 'wss',
		host: env.publicHost,
		clientPort: publicPort,
	};
}

export default defineConfig({
	plugins: [preactPreset(), tailwindcss()],
	server: {
		port: env.port ?? 5173,
		strictPort: isProxyMode || env.port !== undefined,
		...(env.bindAddr !== undefined && { host: env.bindAddr }),
		// Allow only the specific public hostname instead of a wildcard so
		// Vite's origin check stays tight when served through a reverse proxy.
		...(isProxyMode && { allowedHosts: [env.publicHost] }),
		...(hmrConfig !== undefined && { hmr: hmrConfig }),
	},
	test: {
		environment: 'jsdom',
		setupFiles: ['./src/test/setup.ts'],
		fsModuleCache: true,
		coverage: {
			provider: 'v8',
			// Every source file counts, not just the ones a test happened to
			// import — otherwise an untested file is invisible and coverage
			// reads high precisely because nothing exercises it.
			include: ['src/**/*.{ts,tsx}'],
			exclude: [
				...coverageConfigDefaults.exclude,
				// Test scaffolding, not production code.
				'src/test/**',
				// Type declarations only: nothing executable to cover.
				'src/types.ts',
			],
			reporter: ['text', 'html'],
			thresholds: {
				statements: 85,
				branches: 85,
				functions: 85,
				lines: 85,
			},
		},
	},
});
