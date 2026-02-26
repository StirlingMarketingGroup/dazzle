import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ServerStatus from './ServerStatus';
import { useAppStore } from '@/store';

describe('ServerStatus', () => {
  it('shows running status when server is running', () => {
    useAppStore.setState({
      serverRunning: true,
      serverError: null,
      config: { port: 29100, selected_printer: null },
    });

    render(<ServerStatus />);
    expect(screen.getByText('Server running')).toBeInTheDocument();
    expect(screen.getByText('Restart')).toBeInTheDocument();
  });

  it('shows stopped status when server is down', () => {
    useAppStore.setState({
      serverRunning: false,
      serverError: null,
      config: { port: 29100, selected_printer: null },
    });

    render(<ServerStatus />);
    expect(screen.getByText('Server stopped')).toBeInTheDocument();
    expect(screen.getByText('Start')).toBeInTheDocument();
  });

  it('displays server error', () => {
    useAppStore.setState({
      serverRunning: false,
      serverError: 'Port 9100 already in use',
      config: { port: 9100, selected_printer: null },
    });

    render(<ServerStatus />);
    expect(screen.getByText('Port 9100 already in use')).toBeInTheDocument();
  });

  it('shows the port input with current value', () => {
    useAppStore.setState({
      serverRunning: true,
      serverError: null,
      config: { port: 8080, selected_printer: null },
    });

    render(<ServerStatus />);
    const input = screen.getByDisplayValue('8080');
    expect(input).toBeInTheDocument();
  });

  it('calls restartServer on button click', async () => {
    const user = userEvent.setup();
    const restartServer = vi.fn().mockResolvedValue(undefined);

    useAppStore.setState({
      serverRunning: true,
      serverError: null,
      config: { port: 29100, selected_printer: null },
      restartServer,
    });

    render(<ServerStatus />);
    await user.click(screen.getByText('Restart'));

    expect(restartServer).toHaveBeenCalled();
  });

  it('filters non-numeric characters from port input', async () => {
    const user = userEvent.setup();

    useAppStore.setState({
      serverRunning: true,
      serverError: null,
      config: { port: 29100, selected_printer: null },
    });

    render(<ServerStatus />);
    const input = screen.getByDisplayValue('29100');
    await user.clear(input);
    await user.type(input, '80abc80');

    expect(input).toHaveValue('8080');
  });
});
