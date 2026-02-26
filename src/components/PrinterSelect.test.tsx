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

  it('has a refresh button', () => {
    useAppStore.setState({ printers: [] });
    render(<PrinterSelect />);
    expect(screen.getByTitle('Refresh printer list')).toBeInTheDocument();
  });

  it('calls refreshPrinters when clicking refresh', async () => {
    const user = userEvent.setup();
    const refreshPrinters = vi.fn();

    useAppStore.setState({ printers: [], refreshPrinters });

    render(<PrinterSelect />);
    await user.click(screen.getByTitle('Refresh printer list'));

    expect(refreshPrinters).toHaveBeenCalled();
  });
});
