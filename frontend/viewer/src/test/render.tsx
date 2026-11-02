import {render, type RenderOptions} from '@testing-library/preact';
import type {ComponentChildren} from 'preact';
import {LocationProvider} from 'preact-iso';
import {AuthProvider} from '../auth';

export function renderWithLocation(
	ui: ComponentChildren,
	options?: RenderOptions,
) {
	return render(<LocationProvider>{ui}</LocationProvider>, options);
}

export function renderLoggedOut(ui: ComponentChildren, options?: RenderOptions) {
	return render(
		<LocationProvider>
			<AuthProvider>{ui}</AuthProvider>
		</LocationProvider>,
		options,
	);
}

export function renderLoggedIn(ui: ComponentChildren, options?: RenderOptions) {
	return render(
		<LocationProvider>
			<AuthProvider initialEmail="viewer@example.com">{ui}</AuthProvider>
		</LocationProvider>,
		options,
	);
}
