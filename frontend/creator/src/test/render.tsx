import {render, type RenderOptions} from '@testing-library/preact';
import type {ComponentChildren} from 'preact';
import {LocationProvider} from 'preact-iso';
import {AuthProvider} from '../auth';
import {PortalProvider} from '../store';
import type {BankAccount, Website} from '../types';
import {demoWebsites} from './fixtures';

export function renderLoggedOut(ui: ComponentChildren, options?: RenderOptions) {
	return render(
		<LocationProvider>
			<AuthProvider>
				<PortalProvider>{ui}</PortalProvider>
			</AuthProvider>
		</LocationProvider>,
		options,
	);
}

export function renderLoggedIn(
	ui: ComponentChildren,
	data?: {websites?: Website[]; bankAccount?: BankAccount | null},
	options?: RenderOptions,
) {
	return render(
		<LocationProvider>
			<AuthProvider initialEmail="owner@example.com">
				<PortalProvider websites={demoWebsites} {...data}>
					{ui}
				</PortalProvider>
			</AuthProvider>
		</LocationProvider>,
		options,
	);
}
