import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import EducationPage from './EducationPage';

describe('EducationPage Formal Functional Tests', () => {

  it('Requirement 1.1: Default Route Initialization loads Infographics tab', () => {
    render(<EducationPage />);
    
    // Check that the header renders
    expect(screen.getByText('EDUCATION & OUTREACH')).toBeInTheDocument();
    
    // Check that Infographics are displayed by default (Protocol K)
    expect(screen.getByText(/Breaking the Photosynthesis Limit/i)).toBeInTheDocument();
    expect(screen.getByText(/18.2% Eff/i)).toBeInTheDocument();
  });

  it('Requirement 3.2: Education Media Hub Tab Switching to ITER Fusion Master', () => {
    render(<EducationPage />);
    
    // Find the tab buttons
    const iterTabBtn = screen.getByText(/ITER Fusion Master/i);
    
    // Switch to ITER
    fireEvent.click(iterTabBtn);
    
    // Verify ITER specific content is now visible
    expect(screen.getByText('ITER FUSION: DISRUPTIVE PLASMA CONTAINMENT')).toBeInTheDocument();
    expect(screen.getByText(/Tensor-Train Integration/i)).toBeInTheDocument();
    expect(screen.getByText(/320,000x Shrink/i)).toBeInTheDocument();
    
    // Ensure Infographics are no longer visible
    expect(screen.queryByText(/Breaking the Photosynthesis Limit/i)).not.toBeInTheDocument();
  });

  it('Requirement 3.3: Media Kit Download Action triggers download', () => {
    const alertMock = vi.spyOn(window, 'alert').mockImplementation(() => {});
    render(<EducationPage />);
    
    const downloadBtn = screen.getByText(/DOWNLOAD FULL MEDIA KIT/i);
    fireEvent.click(downloadBtn);
    
    expect(alertMock).toHaveBeenCalledWith("Downloading Full Media Kit (High-Res Images, Videos, press release .zip)...");
    alertMock.mockRestore();
  });

});
