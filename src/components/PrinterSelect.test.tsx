import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import PrinterSelect from './PrinterSelect';
import { useAppStore } from '@/store';

describe('PrinterSelect', () => {
  it('shows empty state when no printers', () => {
    useAppStore.setState({ printers: [] });
    render(<PrinterSelect />);
    expect(screen.getByText('No printers found')).toBeInTheDocument();
  });

  it('renders printer list', () => {
    useAppStore.setState({
      printers: [
        { name: 'Zebra ZD420', is_default: true },
        { name: 'Brother QL-800', is_default: false },
      ],
      config: { port: 29100, selected_printer: 'Zebra ZD420' },
    });

    render(<PrinterSelect />);
    expect(screen.getByText('Zebra ZD420')).toBeInTheDocument();
    expect(screen.getByText('Brother QL-800')).toBeInTheDocument();
    expect(screen.getByText('default')).toBeInTheDocument();
  });

  it('calls updateConfig when selecting a printer', async () => {
    const user = userEvent.setup();
    const updateConfig = vi.fn();

    useAppStore.setState({
      printers: [
        { name: 'Zebra ZD420', is_default: true },
        { name: 'Brother QL-800', is_default: false },
      ],
      config: { port: 29100, selected_printer: 'Zebra ZD420' },
      updateConfig,
    });

    render(<PrinterSelect />);
    await user.click(screen.getByText('Brother QL-800'));

    expect(updateConfig).toHaveBeenCalledWith({
      port: 29100,
      selected_printer: 'Brother QL-800',
    });
  });

  it('locks the list to the administrator printer when it is installed', () => {
    useAppStore.setState({
      printers: [
        { name: 'Zebra ZD420', is_default: true },
        { name: 'Brother QL-800', is_default: false },
      ],
      config: { port: 29100, selected_printer: 'Brother QL-800' },
      policyPrinter: 'Zebra ZD420',
    });

    render(<PrinterSelect />);

    const zebra = screen.getByRole('button', { name: /Zebra ZD420/ });
    const brother = screen.getByRole('button', { name: /Brother QL-800/ });
    expect(zebra).toBeDisabled();
    expect(brother).toBeDisabled();
    expect(zebra.className).toContain('bg-app-accent/15');
    expect(brother.className).not.toContain('bg-app-accent/15');
    expect(screen.getByText("Your administrator chose this PC's printer.")).toBeInTheDocument();
  });

  it('says when the administrator printer is not installed', () => {
    useAppStore.setState({
      printers: [{ name: 'Zebra ZD420', is_default: true }],
      config: { port: 29100, selected_printer: 'Zebra ZD420' },
      policyPrinter: 'Warehouse Zebra',
    });

    render(<PrinterSelect />);

    expect(screen.getByRole('button', { name: /Zebra ZD420/ })).toBeDisabled();
    expect(
      screen.getByText(
        'Your administrator chose the printer "Warehouse Zebra", but it is not installed for this user.'
      )
    ).toBeInTheDocument();
  });

  it('has a refresh button', () => {
    useAppStore.setState({ printers: [] });
    render(<PrinterSelect />);
    expect(screen.getByTitle('Refresh printer list')).toBeInTheDocument();
  });

  it('calls refresh when clicking refresh', async () => {
    const user = userEvent.setup();
    const refresh = vi.fn();

    useAppStore.setState({ printers: [], refresh });

    render(<PrinterSelect />);
    await user.click(screen.getByTitle('Refresh printer list'));

    expect(refresh).toHaveBeenCalled();
  });
});
