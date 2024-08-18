Feature: Update Steam

  Scenario: Find the user localconfig.vdf
    Given a computer with Steam installed
    When I scan for config files
    Then we should find 1 or more localconfig.vdf files
