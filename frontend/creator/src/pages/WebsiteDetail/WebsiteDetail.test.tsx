import {describe, expect, it} from 'vitest';
import {WebsiteDetail} from './index';
import {renderLoggedIn} from '../../test/render';
import {metaTag} from '../../api';
import {demoWebsites} from '../../test/fixtures';

describe('WebsiteDetail', () => {
	it('reports an id that belongs to nobody', () => {
		const {getByRole} = renderLoggedIn(<WebsiteDetail id="99"/>);

		expect(getByRole('heading', {name: 'Website not found'})).toBeInTheDocument();
	});

	it('confirms a verified website is connected', () => {
		const {getByText} = renderLoggedIn(<WebsiteDetail id="1"/>);

		expect(
			getByText(/daily-ledger.example is connected to your account/),
		).toBeInTheDocument();
		expect(getByText(/Last checked/)).toBeInTheDocument();
	});

	it('asks for the tag on a website that is not verified', () => {
		const {getByText, queryByText} = renderLoggedIn(<WebsiteDetail id="2"/>);

		expect(getByText(/Add this tag inside the/)).toBeInTheDocument();
		expect(queryByText(/Last checked/)).not.toBeInTheDocument();
	});

	it('shows the meta tag so it can be re-installed', () => {
		const {getByLabelText} = renderLoggedIn(<WebsiteDetail id="1"/>);

		expect(getByLabelText('Meta tag')).toHaveValue(
			metaTag(demoWebsites[0]?.token ?? ''),
		);
	});

	it('summarises reads, readers and earnings', () => {
		const {getByText} = renderLoggedIn(<WebsiteDetail id="1"/>);

		expect(getByText('18,432')).toBeInTheDocument();
		expect(getByText('2,910')).toBeInTheDocument();
		expect(getByText('284.60 EUR')).toBeInTheDocument();
	});

	it('breaks the numbers down by month', () => {
		const {getByText} = renderLoggedIn(<WebsiteDetail id="1"/>);

		expect(getByText('Feb 2024')).toBeInTheDocument();
		expect(getByText('Mar 2024')).toBeInTheDocument();
		expect(getByText('Apr 2024')).toBeInTheDocument();
	});

	it('lists the most read pages, linked out', () => {
		const {getByRole} = renderLoggedIn(<WebsiteDetail id="1"/>);

		const link = getByRole('link', {
			name: "Who really owns the city's water contracts",
		});
		expect(link).toHaveAttribute(
			'href',
			'https://daily-ledger.example/2024/04/water-contracts',
		);
		expect(link).toHaveAttribute('target', '_blank');
	});

	it('says so when a website has no reads yet', () => {
		const {getByText, queryByText} = renderLoggedIn(<WebsiteDetail id="2"/>);

		expect(getByText(/No reads recorded yet/)).toBeInTheDocument();
		expect(queryByText('Most read pages')).not.toBeInTheDocument();
	});

	it('promises reader counts are never reader lists', () => {
		const {getByText} = renderLoggedIn(<WebsiteDetail id="1"/>);

		expect(getByText(/never lists/)).toBeInTheDocument();
	});
});
