% breakup_model.m
% Industrial-grade fragmentation simulation based on NASA Standard Breakup Model (NSBM)
% Generates initial state vectors for collision debris.

function [pos_list, vel_list] = breakup_model(parent_pos, parent_vel, num_pieces, area_to_mass_range)
    % parent_pos: [x, y, z] in km (ECI)
    % parent_vel: [x, y, z] in km/s (ECI)
    % num_pieces: Target count (e.g., 406 or 960)
    
    pos_list = zeros(num_pieces, 3);
    vel_list = zeros(num_pieces, 3);
    
    % Constants for characteristic velocity (simplified NSBM)
    % Delta-V distribution is log-normal
    mu_dv = -1.0; % log(delta_v) mean
    sigma_dv = 0.4; % log(delta_v) std dev
    
    for i = 1:num_pieces
        % Random direction for delta-V
        theta = 2 * pi * rand();
        phi = acos(2 * rand() - 1);
        
        dir = [sin(phi)*cos(theta), sin(phi)*sin(theta), cos(phi)];
        
        % Delta-V magnitude from log-normal distribution (km/s)
        dv_mag = exp(mu_dv + sigma_dv * randn());
        
        % Limit max DV for stability
        dv_mag = min(dv_mag, 0.5); 
        
        pos_list(i, :) = parent_pos; % All start at impact point
        vel_list(i, :) = parent_vel + (dir * dv_mag);
    end
    
    fprintf('Generated %d pieces at [%.2f, %.2f, %.2f]\n', ...
        num_pieces, parent_pos(1), parent_pos(2), parent_pos(3));
end
