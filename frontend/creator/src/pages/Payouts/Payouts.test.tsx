import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {Payouts} from './index';
import {renderLoggedIn} from '../../test/render';
import type {BankAccount} from '../../types';

const ACCOUNT: BankAccount = {
	holder: 'Anna Kovacs',
	iban: 'DE89370400440532013000',
	bic: 'COBADEFFXXX',
};

function fillAccount(
	getByLabelText: (text: string) => HTMLElement,
	iban: string,
	bic = 'COBADEFFXXX',
	holder = 'Anna Kovacs',
) {
	fireEvent.input(getByLabelText('Account holder'), {target: {value: holder}});
	fireEvent.input(getByLabelText('IBAN'), {target: {value: iban}});
	fireEvent.input(getByLabelText('BIC'), {target: {value: bic}});
}

describe('Payouts', () => {
	it('warns when there is nowhere to send the money', () => {
		const {getByText} = renderLoggedIn(<Payouts/>);

		expect(getByText(/We have nowhere to send your money yet/)).toBeInTheDocument();
	});

	it('asks for the holder, IBAN and BIC', () => {
		const {getByLabelText} = renderLoggedIn(<Payouts/>);

		expect(getByLabelText('Account holder')).toBeInTheDocument();
		expect(getByLabelText('IBAN')).toBeInTheDocument();
		expect(getByLabelText('BIC')).toBeInTheDocument();
	});

	it('refuses an unnamed account', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Payouts/>);

		fillAccount(getByLabelText, 'DE89370400440532013000', 'COBADEFFXXX', '');
		fireEvent.click(getByRole('button', {name: 'Save account'}));

		expect(getByText('Enter the name the account is held in.')).toBeInTheDocument();
	});

	it('refuses an IBAN with bad check digits', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Payouts/>);

		fillAccount(getByLabelText, 'DE89370400440532013001');
		fireEvent.click(getByRole('button', {name: 'Save account'}));

		expect(getByText(/check digits do not match/)).toBeInTheDocument();
	});

	it('refuses a non-EU account', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Payouts/>);

		fillAccount(getByLabelText, 'GB82WEST12345698765432');
		fireEvent.click(getByRole('button', {name: 'Save account'}));

		expect(getByText(/not an EU country/)).toBeInTheDocument();
	});

	it('refuses a malformed BIC', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Payouts/>);

		fillAccount(getByLabelText, 'DE89370400440532013000', 'NOPE');
		fireEvent.click(getByRole('button', {name: 'Save account'}));

		expect(getByText(/8 or 11 characters/)).toBeInTheDocument();
	});

	it('saves a valid account and shows it back, grouped', () => {
		const {getByLabelText, getByRole, getByText} = renderLoggedIn(<Payouts/>);

		fillAccount(getByLabelText, 'DE89 3704 0044 0532 0130 00');
		fireEvent.click(getByRole('button', {name: 'Save account'}));

		expect(getByText('Bank account saved.')).toBeInTheDocument();
		expect(getByText('DE89 3704 0044 0532 0130 00')).toBeInTheDocument();
		expect(getByText('Anna Kovacs')).toBeInTheDocument();
	});

	it('shows an account that is already on file instead of the form', () => {
		const {getByRole, getByText, queryByLabelText} = renderLoggedIn(<Payouts/>, {
			bankAccount: ACCOUNT,
		});

		expect(getByText('Anna Kovacs')).toBeInTheDocument();
		expect(queryByLabelText('IBAN')).not.toBeInTheDocument();
		expect(getByRole('button', {name: 'Change account'})).toBeInTheDocument();
	});

	it('reopens the form to change the account', () => {
		const {getByLabelText, getByRole} = renderLoggedIn(<Payouts/>, {
			bankAccount: ACCOUNT,
		});

		fireEvent.click(getByRole('button', {name: 'Change account'}));

		expect(getByLabelText('IBAN')).toHaveValue('DE89 3704 0044 0532 0130 00');
	});

	it('lists every payout with its state', () => {
		const {getAllByRole, getByText} = renderLoggedIn(<Payouts/>);

		expect(getAllByRole('row')).toHaveLength(4);
		expect(getAllByRole('cell', {name: 'Paid'})).toHaveLength(2);
		expect(getByText('Pending')).toBeInTheDocument();
	});

	it('totals what has been paid and what is still owed', () => {
		const {getByText, getAllByText} = renderLoggedIn(<Payouts/>);

		expect(getByText('183.10 EUR')).toBeInTheDocument();
		// Once as the "Next payout" total, once as the pending row itself.
		expect(getAllByText('101.50 EUR')).toHaveLength(2);
	});
});
