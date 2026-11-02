import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {AddWebsite} from './index';
import {renderLoggedIn} from '../../test/render';
import {routeFetch} from '../../test/fixtures';
import {Routes} from '../../routes';
import type {Website} from '../../types';

const created: Website = {
	id: '0b6c4a52-6f43-4b8a-9d3e-1f2a3b4c5d6e',
	domain: 'new-site.example',
	title: 'New Site',
	addedAt: '2026-10-28T12:00:00Z',
	status: 'verified',
	token: 'crv-00112233445566778899aabbccddeeff',
	lastCheckedAt: null,
};

function submit(
	getByLabelText: (text: string) => HTMLElement,
	getByRole: (role: string, options: {name: string}) => HTMLElement,
	domain: string,
	name = '',
) {
	fireEvent.input(getByLabelText('Domain'), {target: {value: domain}});
	if (name !== '') {
		fireEvent.input(getByLabelText('Name'), {target: {value: name}});
	}
	fireEvent.click(getByRole('button', {name: 'Add website'}));
}

describe('AddWebsite', () => {
	it('asks for a domain and an optional name', () => {
		const {getByLabelText} = renderLoggedIn(<AddWebsite/>);

		expect(getByLabelText('Domain')).toBeInTheDocument();
		expect(getByLabelText('Name')).toBeInTheDocument();
	});

	it('rejects something that is not a domain', () => {
		const {getByLabelText, getByRole} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'example');

		expect(getByRole('alert')).toHaveTextContent('does not look like a domain');
	});

	it('refuses a website that was already added', () => {
		const {getByLabelText, getByRole} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'daily-ledger.example');

		expect(getByRole('alert')).toHaveTextContent('already added that website');
	});

	it('sends the bare domain and the name', async () => {
		const fetchMock = routeFetch({'/api/websites': {status: 201, body: created}});
		const {getByLabelText, getByRole, findByRole} = renderLoggedIn(<AddWebsite/>);

		submit(
			getByLabelText,
			getByRole,
			'https://www.new-site.example/blog?ref=1',
			' New Site ',
		);

		expect(
			await findByRole('heading', {name: 'new-site.example is connected'}),
		).toBeInTheDocument();
		expect(fetchMock).toHaveBeenCalledWith(
			'/api/websites',
			expect.objectContaining({
				method: 'POST',
				body: JSON.stringify({domain: 'new-site.example', title: 'New Site'}),
			}),
		);
	});

	it('hands over the meta tag for later ownership checks', async () => {
		routeFetch({'/api/websites': {status: 201, body: created}});
		const {getByLabelText, getByRole, findByLabelText} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'new-site.example');

		expect(await findByLabelText('Meta tag')).toHaveValue(
			`<meta name="creators-reward-site-verification" content="${created.token}">`,
		);
	});

	it('links to the new website and back to the list', async () => {
		routeFetch({'/api/websites': {status: 201, body: created}});
		const {getByLabelText, getByRole, findByRole} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'new-site.example');

		expect(await findByRole('link', {name: 'Open New Site'})).toHaveAttribute(
			'href',
			`/websites/${created.id}`,
		);
		expect(getByRole('link', {name: 'Your websites'})).toHaveAttribute(
			'href',
			Routes.Websites,
		);
	});

	it('refuses a website another account registered', async () => {
		routeFetch({'/api/websites': {status: 409}});
		const {getByLabelText, getByRole, findByRole} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'new-site.example');

		expect(await findByRole('alert')).toHaveTextContent(
			'already registered to an account',
		);
	});

	it('says so when the website could not be added', async () => {
		routeFetch({'/api/websites': {status: 503}});
		const {getByLabelText, getByRole, findByRole} = renderLoggedIn(<AddWebsite/>);

		submit(getByLabelText, getByRole, 'new-site.example');

		expect(await findByRole('alert')).toHaveTextContent('Could not add the website');
		expect(getByRole('button', {name: 'Add website'})).toBeEnabled();
	});
});
