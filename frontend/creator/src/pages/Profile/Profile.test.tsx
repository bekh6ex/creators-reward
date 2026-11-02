import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {Profile} from './index';
import {renderLoggedIn} from '../../test/render';

describe('Profile', () => {
	it('starts from the profile already on file', () => {
		const {getByLabelText} = renderLoggedIn(<Profile/>);

		expect(getByLabelText('Name')).toHaveValue('The Daily Ledger');
		expect(getByLabelText('Homepage')).toHaveValue('https://daily-ledger.example');
		expect(getByLabelText('Contact email')).toHaveValue(
			'editor@daily-ledger.example',
		);
	});

	it('says which fields readers can see', () => {
		const {getByText} = renderLoggedIn(<Profile/>);

		expect(getByText(/Your name and description are public/)).toBeInTheDocument();
	});

	it('refuses an empty name', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Profile/>);

		fireEvent.input(getByLabelText('Name'), {target: {value: '  '}});
		fireEvent.click(getByRole('button', {name: 'Save profile'}));

		expect(getByText(/need a name to recognise you by/)).toBeInTheDocument();
	});

	it('refuses a homepage that is not a full address', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Profile/>);

		fireEvent.input(getByLabelText('Homepage'), {target: {value: 'example.org'}});
		fireEvent.click(getByRole('button', {name: 'Save profile'}));

		expect(getByText(/must be a full address/)).toBeInTheDocument();
	});

	it('refuses a contact address that is not an email', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Profile/>);

		fireEvent.input(getByLabelText('Contact email'), {target: {value: 'nope'}});
		fireEvent.click(getByRole('button', {name: 'Save profile'}));

		expect(getByText(/valid contact email/)).toBeInTheDocument();
	});

	it('accepts an empty homepage', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Profile/>);

		fireEvent.input(getByLabelText('Homepage'), {target: {value: ''}});
		fireEvent.click(getByRole('button', {name: 'Save profile'}));

		expect(getByText('Profile saved.')).toBeInTheDocument();
	});

	it('confirms a saved profile', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Profile/>);

		fireEvent.input(getByLabelText('Name'), {target: {value: 'The Evening Ledger'}});
		fireEvent.input(getByLabelText('Description'), {
			target: {value: 'Local reporting, every evening.'},
		});
		fireEvent.click(getByRole('button', {name: 'Save profile'}));

		expect(getByText('Profile saved.')).toBeInTheDocument();
		expect(getByLabelText('Name')).toHaveValue('The Evening Ledger');
	});
});
